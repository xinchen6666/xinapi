//! 受管路由操作回执、条件发布及账号代理投影的 PostgreSQL owner

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gateway_admin::{
    model::{
        MutationContext, Revision as AdminRevision,
        audit::MutationAuditOperation,
        managed_routes::{
            BeginRoute, ManagedRoute, RouteNode, RouteObservation, RouteOperation, RoutePhase,
        },
        proxies::AccountProxySelection,
    },
    ports::{
        managed_routes::ManagedRouteStore,
        store::{AdminStoreError, AdminStoreErrorKind, AdminStoreResult},
    },
};
use gateway_core::account::OutboundProxy;
use sqlx::{PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

use super::{append_admin_audit_event_in_transaction, bump_config_revision_in_transaction};
use crate::{
    ConflictKind, Revision, StoreError, StoreResult, admin_revision, admin_store_error,
    mutation_audit, postgres_unavailable,
};

const ENTITY: &str = "managed route";

#[derive(Clone)]
pub struct PgManagedRouteStore {
    pool: PgPool,
}
impl PgManagedRouteStore {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct OperationRow {
    operation_id: Uuid,
    route_id: Uuid,
    account_id: String,
    expected_generation: i64,
    expected_proxy_id: Option<String>,
    expected_proxy_revision: Option<i64>,
    expected_proxy_url: Option<String>,
    expected_source_revision: i64,
    candidate: Json<RouteNode>,
    phase: String,
    observation: Option<Json<RouteObservation>>,
    error_code: Option<String>,
    context: Json<MutationContext>,
    published_config_revision: Option<i64>,
    published_proxy_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
impl OperationRow {
    fn view(&self) -> AdminStoreResult<RouteOperation> {
        Ok(RouteOperation {
            operation_id: self.operation_id.to_string(),
            account_id: self.account_id.clone(),
            route_id: self.route_id.to_string(),
            expected_generation: unsigned(self.expected_generation)?,
            expected_proxy_id: self.expected_proxy_id.clone(),
            candidate: self.candidate.0.clone(),
            phase: phase(&self.phase)?,
            observation: self.observation.as_ref().map(|v| v.0.clone()),
            error_code: self.error_code.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct RouteRow {
    route_id: Uuid,
    account_id: String,
    proxy_id: Option<String>,
    generation: i64,
    applying_operation_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct BindingRow {
    outbound_proxy_id: Option<String>,
    outbound_proxy_url: Option<String>,
    proxy_revision: Option<i64>,
    proxy_url: Option<String>,
}

fn invalid(message: &'static str) -> AdminStoreError {
    AdminStoreError::new(AdminStoreErrorKind::Invalid, ENTITY, message)
}
fn conflict() -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::Conflict,
        ENTITY,
        "managed route facts changed",
    )
}
fn missing() -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::NotFound,
        ENTITY,
        "managed route record is absent",
    )
}
fn unavailable(error: sqlx::Error) -> AdminStoreError {
    admin_store_error(
        ENTITY,
        postgres_unavailable("managed route operation", error),
    )
}
fn store_error(error: StoreError) -> AdminStoreError {
    admin_store_error(ENTITY, error)
}
fn uuid(value: &str) -> AdminStoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| invalid("expected UUID identifier"))
}
fn signed(value: u64) -> AdminStoreResult<i64> {
    i64::try_from(value).map_err(|_| invalid("generation exceeds storage range"))
}
fn unsigned(value: i64) -> AdminStoreResult<u64> {
    u64::try_from(value).map_err(|_| invalid("negative generation"))
}
fn phase(value: &str) -> AdminStoreResult<RoutePhase> {
    match value {
        "preparing" => Ok(RoutePhase::Preparing),
        "ready" => Ok(RoutePhase::Ready),
        "applying" => Ok(RoutePhase::Applying),
        "applied" => Ok(RoutePhase::Applied),
        "failed" => Ok(RoutePhase::Failed),
        _ => Err(invalid("unknown operation phase")),
    }
}
async fn lock_config(tx: &mut Transaction<'_, Postgres>) -> AdminStoreResult<()> {
    let value: Option<i64> =
        sqlx::query_scalar("select config_revision from runtime_settings where id = 1 for update")
            .fetch_optional(&mut **tx)
            .await
            .map_err(unavailable)?;
    value.ok_or_else(missing)?;
    Ok(())
}
async fn operation_locked(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AdminStoreResult<OperationRow> {
    sqlx::query_as::<_, OperationRow>(
        "select * from managed_route_operations where operation_id = $1 for update",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(unavailable)?
    .ok_or_else(missing)
}
async fn route_locked(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> AdminStoreResult<RouteRow> {
    sqlx::query_as::<_, RouteRow>("select * from managed_routes where route_id = $1 for update")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(unavailable)?
        .ok_or_else(missing)
}
async fn binding(
    tx: &mut Transaction<'_, Postgres>,
    account: &str,
) -> AdminStoreResult<BindingRow> {
    let row = sqlx::query_as::<_, BindingRow>(
        "select a.outbound_proxy_id, a.outbound_proxy_url, p.revision as proxy_revision, p.proxy_url
         from provider_accounts a left join outbound_proxies p on p.id=a.outbound_proxy_id
         where a.id=$1 for update of a")
        .bind(account).fetch_optional(&mut **tx).await.map_err(unavailable)?.ok_or_else(missing)?;
    if row.outbound_proxy_id.is_some()
        && (row.proxy_revision.is_none() || row.proxy_url != row.outbound_proxy_url)
    {
        return Err(conflict());
    }
    Ok(row)
}
async fn source_revision(
    tx: &mut Transaction<'_, Postgres>,
    source: &str,
) -> AdminStoreResult<i64> {
    sqlx::query_scalar("select revision from subscription_sources where id=$1 for share")
        .bind(uuid(source)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(unavailable)?
        .ok_or_else(missing)
}
async fn verify_expected(
    tx: &mut Transaction<'_, Postgres>,
    op: &OperationRow,
    route: &RouteRow,
) -> AdminStoreResult<()> {
    if route.account_id != op.account_id || route.generation != op.expected_generation {
        return Err(conflict());
    }
    if source_revision(tx, &op.candidate.source_id).await? != op.expected_source_revision {
        return Err(conflict());
    }
    let current = binding(tx, &op.account_id).await?;
    if current.outbound_proxy_id != op.expected_proxy_id
        || current.proxy_revision != op.expected_proxy_revision
        || current.outbound_proxy_url != op.expected_proxy_url
    {
        return Err(conflict());
    }
    Ok(())
}
async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    context: &MutationContext,
    action: MutationAuditOperation,
    entity: &str,
    fields: &[&str],
    revision: Option<Revision>,
) -> AdminStoreResult<()> {
    append_admin_audit_event_in_transaction(
        tx,
        mutation_audit(
            context,
            action,
            entity,
            fields.iter().map(|s| (*s).to_owned()).collect(),
        ),
        revision,
    )
    .await
    .map_err(store_error)
}
async fn finish<T>(
    tx: Transaction<'_, Postgres>,
    result: AdminStoreResult<T>,
) -> AdminStoreResult<T> {
    match result {
        Ok(value) => {
            tx.commit().await.map_err(unavailable)?;
            Ok(value)
        }
        Err(error) => match tx.rollback().await {
            Ok(()) => Err(error),
            Err(cleanup) => Err(error.with_cleanup(cleanup)),
        },
    }
}

#[async_trait]
impl ManagedRouteStore for PgManagedRouteStore {
    async fn begin(
        &self,
        command: BeginRoute,
        context: &MutationContext,
    ) -> AdminStoreResult<(RouteOperation, bool)> {
        let id = uuid(&command.operation_id)?;
        gateway_core::account::ProviderAccountId::new(command.account_id.clone())
            .map_err(|_| invalid("invalid account identifier"))?;
        uuid(&command.candidate.source_id)?;
        uuid(&command.candidate.node_id)?;
        uuid(&command.candidate.version_id)?;
        let generation = signed(command.expected_generation)?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let result = async {
            lock_config(&mut tx).await?;
            if let Some(existing) = sqlx::query_as::<_, OperationRow>("select * from managed_route_operations where operation_id=$1 for update")
                .bind(id).fetch_optional(&mut *tx).await.map_err(unavailable)? {
                if existing.account_id != command.account_id || existing.expected_generation != generation || existing.candidate.0 != command.candidate { return Err(conflict()); }
                return Ok((existing.view()?, false));
            }
            let source_revision = source_revision(&mut tx, &command.candidate.source_id).await?;
            let current = binding(&mut tx, &command.account_id).await?;
            let route = sqlx::query_as::<_, RouteRow>("select * from managed_routes where account_id=$1 for update")
                .bind(&command.account_id).fetch_optional(&mut *tx).await.map_err(unavailable)?;
            let route = match route {
                Some(route) => route,
                None => {
                    if generation != 0 { return Err(conflict()); }
                    sqlx::query_as::<_, RouteRow>("insert into managed_routes(route_id,account_id) values($1,$2) returning *")
                        .bind(Uuid::new_v4()).bind(&command.account_id).fetch_one(&mut *tx).await.map_err(unavailable)?
                }
            };
            if route.generation != generation || route.applying_operation_id.is_some() { return Err(conflict()); }
            let op = sqlx::query_as::<_, OperationRow>(
                "insert into managed_route_operations(operation_id,route_id,account_id,expected_generation,expected_proxy_id,expected_proxy_revision,expected_proxy_url,expected_source_revision,candidate,phase,context)
                 values($1,$2,$3,$4,$5,$6,$7,$8,$9,'preparing',$10) returning *")
                .bind(id).bind(route.route_id).bind(&command.account_id).bind(generation)
                .bind(current.outbound_proxy_id).bind(current.proxy_revision).bind(current.outbound_proxy_url)
                .bind(source_revision).bind(Json(&command.candidate)).bind(Json(context))
                .fetch_one(&mut *tx).await.map_err(unavailable)?;
            audit(&mut tx, context, MutationAuditOperation::ManagedRoutePrepare, &id.to_string(), &["candidate","expected_generation"], None).await?;
            Ok((op.view()?, true))
        }.await;
        finish(tx, result).await
    }

    async fn ready(
        &self,
        operation_id: &str,
        observation: RouteObservation,
    ) -> AdminStoreResult<RouteOperation> {
        if !observation.success {
            return Err(invalid("candidate observation must succeed"));
        }
        let id = uuid(operation_id)?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let result = async {
            lock_config(&mut tx).await?;
            let op = operation_locked(&mut tx, id).await?;
            if op.phase != "preparing" {
                if matches!(op.phase.as_str(), "ready"|"applying"|"applied") && op.observation.as_ref().map(|v| &v.0) == Some(&observation) { return op.view(); }
                return Err(conflict());
            }
            let route = route_locked(&mut tx, op.route_id).await?;
            verify_expected(&mut tx, &op, &route).await?;
            let updated = sqlx::query_as::<_, OperationRow>("update managed_route_operations set phase='ready',observation=$2,error_code=null,updated_at=now() where operation_id=$1 and phase='preparing' returning *")
                .bind(id).bind(Json(observation)).fetch_one(&mut *tx).await.map_err(unavailable)?;
            audit(&mut tx,&op.context.0,MutationAuditOperation::ManagedRouteReady,&id.to_string(),&["phase","observation"],None).await?;
            updated.view()
        }.await;
        finish(tx, result).await
    }

    async fn claim(&self, operation_id: &str) -> AdminStoreResult<RouteOperation> {
        let id = uuid(operation_id)?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let result = async {
            lock_config(&mut tx).await?;
            let op = operation_locked(&mut tx,id).await?;
            if op.phase == "applied" { return op.view(); }
            if !matches!(op.phase.as_str(), "ready"|"applying") || !op.observation.as_ref().is_some_and(|v| v.success) { return Err(conflict()); }
            let route = route_locked(&mut tx,op.route_id).await?;
            verify_expected(&mut tx,&op,&route).await?;
            if route.applying_operation_id.is_some_and(|current| current != id) { return Err(conflict()); }
            if op.phase == "applying" {
                return if route.applying_operation_id == Some(id) { op.view() } else { Err(conflict()) };
            }
            sqlx::query("update managed_routes set applying_operation_id=$2,updated_at=now() where route_id=$1")
                .bind(route.route_id).bind(id).execute(&mut *tx).await.map_err(unavailable)?;
            let updated = sqlx::query_as::<_, OperationRow>("update managed_route_operations set phase='applying',updated_at=now() where operation_id=$1 and phase='ready' returning *")
                .bind(id).fetch_one(&mut *tx).await.map_err(unavailable)?;
            audit(&mut tx,&op.context.0,MutationAuditOperation::ManagedRouteClaim,&id.to_string(),&["phase"],None).await?;
            updated.view()
        }.await;
        finish(tx, result).await
    }

    async fn publish(
        &self,
        operation_id: &str,
        endpoint: OutboundProxy,
        context: &MutationContext,
    ) -> AdminStoreResult<(RouteOperation, AdminRevision)> {
        let id = uuid(operation_id)?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let result = async {
            lock_config(&mut tx).await?;
            let op = operation_locked(&mut tx,id).await?;
            if op.phase == "applied" {
                if op.published_proxy_url.as_deref() != Some(endpoint.expose_url()) { return Err(conflict()); }
                let revision = op.published_config_revision.ok_or_else(||invalid("applied receipt lacks config revision"))?;
                return Ok((op.view()?,AdminRevision::new(unsigned(revision)?).map_err(|_|invalid("invalid config revision"))?));
            }
            if op.phase != "applying" || !op.observation.as_ref().is_some_and(|v|v.success) { return Err(conflict()); }
            let route = route_locked(&mut tx,op.route_id).await?;
            if route.applying_operation_id != Some(id) { return Err(conflict()); }
            verify_expected(&mut tx,&op,&route).await?;
            let next = route.generation.checked_add(1).ok_or_else(||invalid("generation exhausted"))?;
            let proxy_id = route.proxy_id.clone().unwrap_or_else(||format!("proxy_managed_{}",route.route_id.simple()));
            let collision: bool = sqlx::query_scalar("select exists(select 1 from outbound_proxies where proxy_url=$1 and id<>$2)")
                .bind(endpoint.expose_url()).bind(&proxy_id).fetch_one(&mut *tx).await.map_err(unavailable)?;
            if collision { return Err(conflict()); }
            if route.proxy_id.is_some() {
                let refs: i64 = sqlx::query_scalar("select count(*) from provider_accounts where outbound_proxy_id=$1 and id<>$2")
                    .bind(&proxy_id).bind(&op.account_id).fetch_one(&mut *tx).await.map_err(unavailable)?;
                if refs != 0 { return Err(conflict()); }
                let changed = sqlx::query("update outbound_proxies set proxy_url=$2,route_generation=$3,revision=revision+1,enabled=true,updated_at=now(),last_test_at=now(),last_test_success=true,last_test_latency_ms=$4,last_test_ip=$5,last_test_ipv4=$6,last_test_ipv6=$7,last_test_message='Managed route candidate verified' where id=$1")
                    .bind(&proxy_id).bind(endpoint.expose_url()).bind(next)
                    .bind(signed(op.observation.as_ref().expect("checked observation").latency_ms)?)
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ip).map(|v|v.to_string()))
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ipv4).map(|v|v.to_string()))
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ipv6).map(|v|v.to_string()))
                    .execute(&mut *tx).await.map_err(unavailable)?;
                if changed.rows_affected()!=1{return Err(conflict());}
            } else {
                sqlx::query("insert into outbound_proxies(id,name,proxy_url,route_generation,enabled,last_test_at,last_test_success,last_test_latency_ms,last_test_ip,last_test_ipv4,last_test_ipv6,last_test_message) values($1,$2,$3,$4,true,now(),true,$5,$6,$7,$8,'Managed route candidate verified')")
                    .bind(&proxy_id).bind(format!("Managed route {}",route.route_id.simple())).bind(endpoint.expose_url()).bind(next)
                    .bind(signed(op.observation.as_ref().expect("checked observation").latency_ms)?)
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ip).map(|v|v.to_string()))
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ipv4).map(|v|v.to_string()))
                    .bind(op.observation.as_ref().and_then(|v|v.exit_ipv6).map(|v|v.to_string()))
                    .execute(&mut *tx).await.map_err(unavailable)?;
            }
            // 账号关联与解析URL共同发布，换代同时撤销旧出口额度缓存
            let changed=sqlx::query("update provider_accounts set managed_egress_required=true,outbound_proxy_id=$2,outbound_proxy_url=$3,(provider_quota_json,quota_observed_at)=(select provider_quota_json,quota_observed_at where quota_paused),updated_at=greatest(now(),updated_at) where id=$1 and outbound_proxy_id is not distinct from $4 and outbound_proxy_url is not distinct from $5")
                .bind(&op.account_id).bind(&proxy_id).bind(endpoint.expose_url()).bind(&op.expected_proxy_id).bind(&op.expected_proxy_url)
                .execute(&mut *tx).await.map_err(unavailable)?;
            if changed.rows_affected()!=1{return Err(conflict());}
            sqlx::query("update managed_routes set proxy_id=$2,generation=$3,current_node=$4,observation=$5,applying_operation_id=null,updated_at=now() where route_id=$1")
                .bind(route.route_id).bind(&proxy_id).bind(next).bind(&op.candidate).bind(&op.observation)
                .execute(&mut *tx).await.map_err(unavailable)?;
            let revision=bump_config_revision_in_transaction(&mut tx).await.map_err(store_error)?;
            let updated=sqlx::query_as::<_,OperationRow>("update managed_route_operations set phase='applied',published_config_revision=$2,published_proxy_url=$3,error_code=null,updated_at=now() where operation_id=$1 and phase='applying' returning *")
                .bind(id).bind(signed(revision.get())?).bind(endpoint.expose_url()).fetch_one(&mut *tx).await.map_err(unavailable)?;
            audit(&mut tx,context,MutationAuditOperation::ManagedRoutePublish,&route.route_id.to_string(),&["proxy_id","generation","current_node"],Some(revision)).await?;
            audit(&mut tx,context,MutationAuditOperation::ProviderAccountUpdate,&op.account_id,&["outbound_proxy"],Some(revision)).await?;
            Ok((updated.view()?,admin_revision(revision)?))
        }.await;
        finish(tx, result).await
    }

    async fn fail(&self, operation_id: &str, reason: &str) -> AdminStoreResult<RouteOperation> {
        if reason.is_empty()
            || reason.len() > 128
            || !reason
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(invalid("failure requires a stable code"));
        }
        let id = uuid(operation_id)?;
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        let result=async{
            lock_config(&mut tx).await?;let op=operation_locked(&mut tx,id).await?;
            if matches!(op.phase.as_str(),"applied"|"failed"){return op.view();}
            if !matches!(op.phase.as_str(),"preparing"|"ready"|"applying"){return Err(conflict());}
            sqlx::query("update managed_routes set applying_operation_id=null,updated_at=now() where route_id=$1 and applying_operation_id=$2")
                .bind(op.route_id).bind(id).execute(&mut *tx).await.map_err(unavailable)?;
            let updated=sqlx::query_as::<_,OperationRow>("update managed_route_operations set phase='failed',error_code=$2,updated_at=now() where operation_id=$1 returning *")
                .bind(id).bind(reason).fetch_one(&mut *tx).await.map_err(unavailable)?;
            audit(&mut tx,&op.context.0,MutationAuditOperation::ManagedRouteFail,&id.to_string(),&["phase","error_code"],None).await?;
            updated.view()
        }.await;
        finish(tx, result).await
    }

    async fn operation(&self, operation_id: &str) -> AdminStoreResult<RouteOperation> {
        let row = sqlx::query_as::<_, OperationRow>(
            "select * from managed_route_operations where operation_id=$1",
        )
        .bind(uuid(operation_id)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(unavailable)?
        .ok_or_else(missing)?;
        row.view()
    }

    async fn route(&self, account_id: &str) -> AdminStoreResult<Option<ManagedRoute>> {
        #[derive(sqlx::FromRow)]
        struct ViewRow {
            route_id: Uuid,
            account_id: String,
            proxy_id: Option<String>,
            account_proxy_id: Option<String>,
            generation: i64,
            current_node: Option<Json<RouteNode>>,
            observation: Option<Json<RouteObservation>>,
            applying_operation_id: Option<Uuid>,
            proxy_url: Option<String>,
            updated_at: DateTime<Utc>,
        }
        let row=sqlx::query_as::<_,ViewRow>("select r.route_id,r.account_id,r.proxy_id,a.outbound_proxy_id as account_proxy_id,r.generation,r.current_node,r.observation,r.applying_operation_id,p.proxy_url,r.updated_at from managed_routes r join provider_accounts a on a.id=r.account_id left join outbound_proxies p on p.id=r.proxy_id where r.account_id=$1")
            .bind(account_id).fetch_optional(&self.pool).await.map_err(unavailable)?;
        row.map(|r| {
            Ok(ManagedRoute {
                route_id: r.route_id.to_string(),
                account_id: r.account_id,
                proxy_id: r.proxy_id,
                account_proxy_id: r.account_proxy_id,
                generation: unsigned(r.generation)?,
                current: r.current_node.map(|v| v.0),
                observation: r.observation.map(|v| v.0),
                applying_operation_id: r.applying_operation_id.map(|v| v.to_string()),
                endpoint: r
                    .proxy_url
                    .map(|url| {
                        OutboundProxy::parse(&url)
                            .map(|p| p.endpoint())
                            .map_err(|_| invalid("invalid route endpoint"))
                    })
                    .transpose()?,
                updated_at: r.updated_at,
            })
        })
        .transpose()
    }
}

/// 通用代理编辑只处理普通代理，受管代理由发布事务独占
pub(crate) async fn guard_proxy_mutation(
    tx: &mut Transaction<'_, Postgres>,
    id: &str,
) -> StoreResult<()> {
    let managed: bool =
        sqlx::query_scalar("select exists(select 1 from managed_routes where proxy_id=$1) or exists(select 1 from managed_route_operations where expected_proxy_id=$1 and phase='applying')")
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| postgres_unavailable("check managed proxy mutation", e))?;
    if managed {
        return Err(route_conflict(id));
    }
    Ok(())
}

/// 正在应用的账号关系保持固定，其他账号始终拒绝借用受管入口
pub(crate) async fn guard_account_binding(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[String],
    selection: &AccountProxySelection,
) -> StoreResult<()> {
    let applying:bool=sqlx::query_scalar("select exists(select 1 from managed_routes where account_id=any($1::text[]) and applying_operation_id is not null)")
        .bind(ids).fetch_one(&mut **tx).await.map_err(|e|postgres_unavailable("check managed account operation",e))?;
    if applying {
        return Err(route_conflict("applying"));
    }
    if let AccountProxySelection::Saved(proxy) = selection {
        let owner: Option<String> =
            sqlx::query_scalar("select account_id from managed_routes where proxy_id=$1")
                .bind(proxy)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| postgres_unavailable("check managed proxy owner", e))?;
        if owner.is_some_and(|owner| ids.iter().any(|id| id != &owner)) {
            return Err(route_conflict(proxy));
        }
    }
    Ok(())
}

pub(crate) async fn guard_account_delete(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[String],
) -> StoreResult<()> {
    let managed:bool=sqlx::query_scalar("select exists(select 1 from managed_routes r where r.account_id=any($1::text[]) and (r.proxy_id is not null or exists(select 1 from managed_route_operations o where o.route_id=r.route_id and o.phase in ('preparing','ready','applying'))))")
        .bind(ids).fetch_one(&mut **tx).await.map_err(|e|postgres_unavailable("check managed account deletion",e))?;
    if managed {
        return Err(route_conflict("managed_owner"));
    }
    Ok(())
}
fn route_conflict(id: &str) -> StoreError {
    StoreError::Conflict {
        source: None,
        entity: ENTITY,
        id: id.to_owned(),
        kind: ConflictKind::InvalidTransition,
    }
}
