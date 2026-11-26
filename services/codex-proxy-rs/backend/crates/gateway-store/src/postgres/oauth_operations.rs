//! 授权操作的持久领取、冻结路由条件与完成事实查询

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gateway_admin::{
    model::{
        MutationActor, MutationContext,
        audit::{MutationAuditIntent, MutationAuditOperation},
        managed_routes::{RouteNode, RouteObservation},
        oauth_operations::{OAuthOperation, StartOAuthOperation},
        provider_credentials::AuthorizationStarted,
    },
    ports::{
        oauth_operations::{OAuthEngineReservation, OAuthOperationStore},
        store::{AdminStoreError, AdminStoreErrorKind, AdminStoreResult},
    },
};
use sqlx::{PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

#[derive(Clone)]
pub struct PgOAuthOperationStore {
    pool: PgPool,
}
impl PgOAuthOperationStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
#[derive(sqlx::FromRow)]
pub(crate) struct OperationRow {
    pub(crate) operation_id: Uuid,
    pub(crate) request: Json<StartOAuthOperation>,
    pub(crate) owner: Json<MutationActor>,
    pub(crate) planned_account_id: String,
    pub(crate) is_new: bool,
    pub(crate) provider_kind: String,
    pub(crate) node: Json<RouteNode>,
    pub(crate) observation: Option<Json<RouteObservation>>,
    pub(crate) expected_source_revision: i64,
    pub(crate) proxy_id: String,
    pub(crate) expected_proxy_revision: Option<i64>,
    pub(crate) route_generation: i64,
    pub(crate) endpoint_url: Option<String>,
    pub(crate) phase: String,
    pub(crate) flow_id: Option<String>,
    pub(crate) authorization_url: Option<String>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) reason: Option<String>,
}
impl OperationRow {
    pub(crate) fn check_owner(&self, owner: &MutationActor) -> AdminStoreResult<()> {
        if self.owner.0 == *owner {
            Ok(())
        } else {
            Err(conflict())
        }
    }
    fn has_settled_exchange(&self) -> bool {
        self.phase == "completing" && self.reason.as_deref() == Some("completion_unknown")
    }

    fn expired_reservation(&self) -> bool {
        self.is_new
            && self.expires_at <= Utc::now()
            && reclaimable_phase(&self.phase, self.reason.as_deref())
    }

    pub(crate) fn view(self, receipt_confirmed: bool) -> AdminStoreResult<OAuthOperation> {
        Ok(OAuthOperation {
            request: self.request.0,
            owner: self.owner.0,
            planned_account_id: self.planned_account_id,
            node: self.node.0,
            observation: self.observation.map(|value| value.0),
            source_revision: unsigned(self.expected_source_revision)?,
            proxy_id: self.proxy_id,
            proxy_revision: self.expected_proxy_revision.map(unsigned).transpose()?,
            route_generation: unsigned(self.route_generation)?,
            endpoint: self.endpoint_url,
            phase: self.phase,
            flow_id: self.flow_id,
            authorization_url: self.authorization_url,
            expires_at: self.expires_at,
            reason: self.reason,
            receipt_confirmed,
        })
    }
}
fn reclaimable_phase(phase: &str, reason: Option<&str>) -> bool {
    matches!(phase, "pending" | "failed" | "cancelled")
        || (phase == "completing" && reason == Some("completion_unknown"))
}

pub(crate) fn conflict() -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::Conflict,
        "oauth operation",
        "授权归属或路由条件已变化，请读取当前回执",
    )
}
fn unavailable(e: sqlx::Error) -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::Unavailable,
        "oauth operation",
        "授权操作存储暂时失联",
    )
    .with_source(e)
}
fn unsigned(n: i64) -> AdminStoreResult<u64> {
    u64::try_from(n).map_err(|_| conflict())
}
fn signed(n: u64) -> AdminStoreResult<i64> {
    i64::try_from(n).map_err(|_| conflict())
}
fn id(value: &str) -> AdminStoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| conflict())
}
pub(crate) async fn lock_config(tx: &mut Transaction<'_, Postgres>) -> AdminStoreResult<()> {
    sqlx::query("select config_revision from runtime_settings where id=1 for update")
        .execute(&mut **tx)
        .await
        .map_err(unavailable)?;
    Ok(())
}
pub(crate) async fn locked(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> AdminStoreResult<OperationRow> {
    locked_operation(tx, operation_id)
        .await?
        .ok_or_else(conflict)
}

async fn locked_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> AdminStoreResult<Option<OperationRow>> {
    sqlx::query_as("select * from managed_oauth_operations where operation_id=$1 for update")
        .bind(id(operation_id)?)
        .fetch_optional(&mut **tx)
        .await
        .map_err(unavailable)
}

const RESERVATION_HAS_REFERENCES: &str = "(
    exists(select 1 from provider_accounts a where a.id=o.planned_account_id or a.outbound_proxy_id=o.proxy_id)
    or exists(select 1 from managed_routes r where r.account_id=o.planned_account_id or r.proxy_id=o.proxy_id)
    or exists(select 1 from outbound_proxies p where p.id=o.proxy_id)
)";

async fn require_zero_references(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: Uuid,
) -> AdminStoreResult<()> {
    let sql = format!(
        "select {RESERVATION_HAS_REFERENCES} from managed_oauth_operations o where o.operation_id=$1"
    );
    let referenced: bool = sqlx::query_scalar(&sql)
        .bind(operation_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
    if referenced {
        return Err(conflict());
    }
    Ok(())
}

async fn set_cancelled(
    tx: &mut Transaction<'_, Postgres>,
    row: OperationRow,
) -> AdminStoreResult<OperationRow> {
    if row.phase == "cancelled" {
        return Ok(row);
    }
    sqlx::query_as("update managed_oauth_operations set phase='cancelled',authorization_url=null,updated_at=now() where operation_id=$1 returning *")
        .bind(row.operation_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)
}
/// 配置写入沿相同全局锁，授权提交随后锁定相关来源与账号事实
pub(crate) async fn validate_frozen(
    tx: &mut Transaction<'_, Postgres>,
    row: &OperationRow,
) -> AdminStoreResult<()> {
    if row.expires_at <= Utc::now()
        || (matches!(row.phase.as_str(), "pending" | "completing")
            && !row.observation.as_ref().is_some_and(|value| value.success))
    {
        return Err(conflict());
    }
    let source: Option<i64> =
        sqlx::query_scalar("select revision from subscription_sources where id=$1 for share")
            .bind(id(&row.node.source_id)?)
            .fetch_optional(&mut **tx)
            .await
            .map_err(unavailable)?;
    if source != Some(row.expected_source_revision) {
        return Err(conflict());
    }
    if row.is_new {
        let exists:bool=sqlx::query_scalar("select exists(select 1 from provider_accounts where id=$1) or exists(select 1 from managed_routes where account_id=$1) or exists(select 1 from outbound_proxies where id=$2)").bind(&row.planned_account_id).bind(&row.proxy_id).fetch_one(&mut **tx).await.map_err(unavailable)?;
        if exists {
            return Err(conflict());
        }
    } else {
        let valid:bool=sqlx::query_scalar("select exists(select 1 from provider_accounts a join outbound_proxies p on p.id=a.outbound_proxy_id join managed_routes r on r.account_id=a.id where a.id=$1 and a.provider_kind=$2 and a.enabled and a.managed_egress_required and a.outbound_proxy_url=$3 and p.id=$4 and p.proxy_url=$3 and p.enabled and p.revision=$5 and p.route_generation=$6 and r.proxy_id=p.id and r.generation=$6 and r.current_node=$7 and r.applying_operation_id is null)")
            .bind(&row.planned_account_id).bind(&row.provider_kind).bind(&row.endpoint_url).bind(&row.proxy_id).bind(row.expected_proxy_revision).bind(row.route_generation).bind(&row.node).fetch_one(&mut **tx).await.map_err(unavailable)?;
        if !valid {
            return Err(conflict());
        }
    }
    Ok(())
}
#[async_trait]
impl OAuthOperationStore for PgOAuthOperationStore {
    async fn owns_flow(&self, flow_id: &str) -> AdminStoreResult<bool> {
        sqlx::query_scalar("select exists(select 1 from managed_oauth_operations where flow_id=$1)")
            .bind(flow_id)
            .fetch_one(&self.pool)
            .await
            .map_err(unavailable)
    }

    async fn begin(&self, op: OAuthOperation) -> AdminStoreResult<(OAuthOperation, bool)> {
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        lock_config(&mut tx).await?;
        let inserted=sqlx::query("insert into managed_oauth_operations(operation_id,request,owner,planned_account_id,is_new,provider_kind,node,expected_source_revision,proxy_id,expected_proxy_revision,route_generation,endpoint_url,phase,expires_at) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'preparing',$13) on conflict(operation_id) do nothing")
            .bind(id(&op.request.operation_id)?).bind(Json(&op.request)).bind(Json(&op.owner)).bind(&op.planned_account_id).bind(op.is_new()).bind(&op.request.provider).bind(Json(&op.node)).bind(signed(op.source_revision)?).bind(&op.proxy_id).bind(op.proxy_revision.map(signed).transpose()?).bind(signed(op.route_generation)?).bind(&op.endpoint).bind(op.expires_at).execute(&mut *tx).await.map_err(unavailable)?.rows_affected()==1;
        let row = locked(&mut tx, &op.request.operation_id).await?;
        row.check_owner(&op.owner)?;
        if row.request.0 != op.request {
            return Err(conflict());
        }
        if inserted {
            validate_frozen(&mut tx, &row).await?;
        }
        let result = row.view(false)?;
        tx.commit().await.map_err(unavailable)?;
        Ok((result, inserted))
    }
    async fn get(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<Option<OAuthOperation>> {
        let row: Option<OperationRow> =
            sqlx::query_as("select * from managed_oauth_operations where operation_id=$1")
                .bind(id(operation_id)?)
                .fetch_optional(&self.pool)
                .await
                .map_err(unavailable)?;
        let Some(row) = row else { return Ok(None) };
        row.check_owner(owner)?;
        let confirmed:bool=sqlx::query_scalar("select exists(select 1 from managed_oauth_operations o join provider_authorization_receipts r on r.provider_kind=o.provider_kind and r.flow_digest=o.receipt_flow_digest and r.owner_digest=o.receipt_owner_digest where o.operation_id=$1 and o.phase='completed' and r.expires_at>now() and jsonb_array_length(r.accounts_json)=1 and r.accounts_json->0->>'account_id'=o.planned_account_id)").bind(row.operation_id).fetch_one(&self.pool).await.map_err(unavailable)?;
        Ok(Some(row.view(confirmed)?))
    }
    async fn endpoint(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        endpoint: &str,
    ) -> AdminStoreResult<()> {
        let changed=sqlx::query("update managed_oauth_operations set endpoint_url=$3,updated_at=now() where operation_id=$1 and owner=$2 and phase='preparing' and expires_at>now() and (endpoint_url is null or endpoint_url=$3)").bind(id(operation_id)?).bind(Json(owner)).bind(endpoint).execute(&self.pool).await.map_err(unavailable)?.rows_affected();
        if changed == 1 {
            Ok(())
        } else {
            Err(conflict())
        }
    }
    async fn ready(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        started: AuthorizationStarted,
        observation: RouteObservation,
    ) -> AdminStoreResult<OAuthOperation> {
        if !observation.success {
            return Err(conflict());
        }
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        lock_config(&mut tx).await?;
        let row = locked(&mut tx, operation_id).await?;
        row.check_owner(owner)?;
        validate_frozen(&mut tx, &row).await?;
        if row.phase != "preparing" || row.endpoint_url.is_none() {
            return Err(conflict());
        }
        let row:OperationRow=sqlx::query_as("update managed_oauth_operations set phase='pending',flow_id=$2,authorization_url=$3,expires_at=least(expires_at,$4),observation=$5,updated_at=now() where operation_id=$1 returning *").bind(row.operation_id).bind(started.flow_id).bind(started.authorization_url).bind(started.expires_at).bind(Json(observation)).fetch_one(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        row.view(false)
    }
    async fn claim(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<Option<OAuthOperation>> {
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        lock_config(&mut tx).await?;
        let row = locked(&mut tx, operation_id).await?;
        row.check_owner(owner)?;
        if row.phase != "pending" {
            return Ok(None);
        }
        validate_frozen(&mut tx, &row).await?;
        let row:OperationRow=sqlx::query_as("update managed_oauth_operations set phase='completing',authorization_url=null,updated_at=now() where operation_id=$1 and phase='pending' returning *").bind(row.operation_id).fetch_one(&mut *tx).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(Some(row.view(false)?))
    }
    async fn fail(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        reason: &str,
    ) -> AdminStoreResult<()> {
        sqlx::query("update managed_oauth_operations set phase=case when phase='preparing' then 'failed' else phase end,reason=$3,authorization_url=null,updated_at=now() where operation_id=$1 and owner=$2 and phase in ('preparing','completing')").bind(id(operation_id)?).bind(Json(owner)).bind(reason).execute(&self.pool).await.map_err(unavailable)?;
        Ok(())
    }
    async fn cancel(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<OAuthOperation> {
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        lock_config(&mut tx).await?;
        let row = locked(&mut tx, operation_id).await?;
        row.check_owner(owner)?;
        let settled_unknown = row.phase == "completing"
            && (row.expires_at <= Utc::now() || row.has_settled_exchange());
        if !matches!(
            row.phase.as_str(),
            "preparing" | "pending" | "failed" | "cancelled"
        ) && !settled_unknown
        {
            return Err(conflict());
        }
        // 新预留以及未知交换的回收先确认全部持久引用已释放
        if row.is_new || settled_unknown {
            require_zero_references(&mut tx, row.operation_id).await?;
        }
        let row = set_cancelled(&mut tx, row).await?;
        tx.commit().await.map_err(unavailable)?;
        row.view(false)
    }

    async fn engine_reservations(
        &self,
        account_ids: &[String],
    ) -> AdminStoreResult<Vec<OAuthEngineReservation>> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "select o.operation_id, o.planned_account_id, o.expires_at, o.phase, o.reason,
                    {RESERVATION_HAS_REFERENCES}
             from managed_oauth_operations o where o.is_new and o.phase <> 'completed'
             and o.planned_account_id = any($1)"
        );
        let rows: Vec<(Uuid, String, DateTime<Utc>, String, Option<String>, bool)> =
            sqlx::query_as(&sql)
                .bind(account_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(unavailable)?;
        let now = Utc::now();
        Ok(rows
            .into_iter()
            .map(
                |(id, account_id, expires_at, phase, reason, referenced)| OAuthEngineReservation {
                    operation_id: id.to_string(),
                    account_id,
                    expires_at,
                    reclaimable: expires_at <= now
                        && !referenced
                        && reclaimable_phase(&phase, reason.as_deref()),
                },
            )
            .collect())
    }

    async fn cancel_expired_reservation(
        &self,
        operation_id: &str,
        context: &MutationContext,
    ) -> AdminStoreResult<OAuthOperation> {
        let mut tx = self.pool.begin().await.map_err(unavailable)?;
        lock_config(&mut tx).await?;
        let row = locked_operation(&mut tx, operation_id)
            .await?
            .ok_or_else(|| {
                AdminStoreError::new(
                    AdminStoreErrorKind::NotFound,
                    "oauth operation",
                    "授权预留记录缺失",
                )
            })?;
        if !row.expired_reservation() {
            return Err(conflict());
        }
        require_zero_references(&mut tx, row.operation_id).await?;
        let changed = row.phase != "cancelled";
        let row = set_cancelled(&mut tx, row).await?;
        if changed {
            let audit = MutationAuditIntent {
                operation: MutationAuditOperation::OAuthReservationReclaim,
                entity_ref: operation_id,
            }
            .event(context, vec!["phase".to_owned()]);
            super::admin_security_audit::append_admin_audit_event_in_transaction(
                &mut tx, audit, None,
            )
            .await
            .map_err(|error| {
                AdminStoreError::new(
                    AdminStoreErrorKind::Unavailable,
                    "oauth operation",
                    "授权回收审计写入失败",
                )
                .with_source(error)
            })?;
        }
        tx.commit().await.map_err(unavailable)?;
        row.view(false)
    }

    async fn reclaimable(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<bool> {
        let sql = format!(
            "select exists(select 1 from managed_oauth_operations o where operation_id=$1 and owner=$2 and is_new and phase in ('failed','cancelled') and not {RESERVATION_HAS_REFERENCES})"
        );
        sqlx::query_scalar(&sql)
            .bind(id(operation_id)?)
            .bind(Json(owner))
            .fetch_one(&self.pool)
            .await
            .map_err(unavailable)
    }
}
