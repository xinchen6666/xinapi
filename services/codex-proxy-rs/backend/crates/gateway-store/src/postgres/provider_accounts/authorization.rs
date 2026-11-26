//! 完成回执与账号、审计共享提交边界；Redis claim 只负责上游准备阶段的互斥

use gateway_admin::model::provider_credentials::{
    AuthorizationCommitResult, AuthorizationMutationTarget, AuthorizationReceiptKey,
    CredentialMutationResult,
};
use serde::{Deserialize, Serialize};

use super::{admin_adapter, repository, *};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredAccount {
    account_id: String,
    credential_revision: Option<u64>,
}

impl PgAdminAccountStore {
    pub(super) async fn load_authorization_receipt(
        &self,
        key: &AuthorizationReceiptKey,
    ) -> AdminStoreResult<Option<CredentialMutationResult>> {
        load_receipt(&self.pool, key).await
    }

    pub(super) async fn commit_authorization_once(
        &self,
        command: AuthorizationCommit,
        context: &MutationContext,
    ) -> AdminStoreResult<AuthorizationCommitResult> {
        if command.key.provider_kind() != command.pending.provider_kind()
            || !command.key.matches_context(context)
            || !command.pending.owner_binding().matches_context(context)
        {
            return Err(invalid_receipt());
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|source| unavailable_receipt().with_source(source))?;
        let outcome = commit_authorization_in_transaction(&mut transaction, command, context).await;
        match outcome {
            Ok(outcome) => {
                transaction
                    .commit()
                    .await
                    .map_err(|source| unavailable_receipt().with_source(source))?;
                if let Some(revision) = outcome.result.credential_revision {
                    self.accounts
                        .advance_protection_revision(
                            outcome.result.account_id.as_str(),
                            store_revision(revision)?,
                        )
                        .await;
                }
                Ok(outcome)
            }
            Err(error) => {
                let error = match transaction.rollback().await {
                    Ok(()) => error,
                    Err(cleanup) => error.with_cleanup(cleanup),
                };
                Err(error)
            }
        }
    }
}

async fn commit_authorization_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    mut command: AuthorizationCommit,
    context: &MutationContext,
) -> AdminStoreResult<AuthorizationCommitResult> {
    let key = &command.key;
    // 同一 flow 的竞争者先等待事务结果；Redis 租约过期不能造成第二次账号提交
    let lock = format!(
        "authorization:{}:{}",
        key.provider_kind(),
        key.flow_digest()
    );
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1, 814037))")
        .bind(lock)
        .execute(&mut **transaction)
        .await
        .map_err(|source| unavailable_receipt().with_source(source))?;
    if let Some(result) = load_receipt(&mut **transaction, key).await? {
        return Ok(AuthorizationCommitResult {
            result,
            newly_committed: false,
        });
    }
    let managed_operation = if let Some(operation_id) =
        command.pending.managed_authorization().map(str::to_owned)
    {
        super::super::oauth_operations::lock_config(transaction).await?;
        let row = super::super::oauth_operations::locked(transaction, &operation_id).await?;
        row.check_owner(&context.actor)?;
        if row.phase != "completing" || row.provider_kind != key.provider_kind().as_str() {
            return Err(invalid_receipt());
        }
        let expected = AuthorizationReceiptKey::new(
            key.provider_kind().clone(),
            row.flow_id.as_deref().ok_or_else(invalid_receipt)?,
            context,
        )
        .map_err(|_| invalid_receipt())?;
        if expected != *key {
            return Err(invalid_receipt());
        }
        super::super::oauth_operations::validate_frozen(transaction, &row).await?;
        let endpoint = row.endpoint_url.as_deref().ok_or_else(invalid_receipt)?;
        if command.pending.outbound_proxy().map(|p| p.expose_url()) != Some(endpoint)
            || command.pending.outbound_proxy_id() != Some(row.proxy_id.as_str())
        {
            return Err(invalid_receipt());
        }
        if row.is_new {
            let AuthorizationCredentialCommit::Create(credential) = &mut command.credential else {
                return Err(invalid_receipt());
            };
            if !matches!(
                command.pending.target(),
                AuthorizationMutationTarget::Create { .. }
            ) {
                return Err(invalid_receipt());
            }
            let proxy = gateway_core::account::OutboundProxy::parse(endpoint)
                .map_err(|_| invalid_receipt())?;
            credential.account_id = CoreProviderAccountId::new(row.planned_account_id.clone())
                .map_err(|_| invalid_receipt())?;
            credential.outbound_proxy = Some(proxy.clone());
            sqlx::query("insert into outbound_proxies(id,name,proxy_url,enabled,route_generation) values($1,$2,$3,true,0)")
                .bind(&row.proxy_id).bind(format!("Authorized route {}",row.operation_id.simple())).bind(endpoint)
                .execute(&mut **transaction).await.map_err(|e|unavailable_receipt().with_source(e))?;
            command.pending = command.pending.clone().with_outbound_proxy(Some(proxy));
        } else if !matches!(command.pending.target(),AuthorizationMutationTarget::Reauthorize{account_id} if account_id.as_str()==row.planned_account_id)
        {
            return Err(invalid_receipt());
        }
        Some(row)
    } else {
        None
    };
    let result = match (command.pending.target(), command.credential) {
        (
            AuthorizationMutationTarget::Create { .. },
            AuthorizationCredentialCommit::Create(credential),
        ) => {
            if &credential.provider_kind != key.provider_kind() {
                return Err(invalid_receipt());
            }
            let prepared = admin_adapter::prepare_import(
                PreparedCredentialImport {
                    provider_kind: key.provider_kind().clone(),
                    credentials: vec![*credential],
                },
                command.settings,
                context,
                gateway_admin::model::audit::MutationAuditOperation::ProviderAccountAuthorize,
                command
                    .pending
                    .outbound_proxy_id()
                    .zip(command.pending.outbound_proxy())
                    .map(
                        |(id, proxy)| gateway_admin::model::proxies::ImportProxyBinding {
                            id: id.into(),
                            proxy: proxy.clone(),
                        },
                    ),
            )?;
            let result = repository::import_provider_accounts_in_transaction(transaction, prepared)
                .await
                .map_err(|error| admin_store_error(ENTITY, error))?;
            admin_adapter::authorization_import_result(result)?
        }
        (
            AuthorizationMutationTarget::Reauthorize { account_id },
            AuthorizationCredentialCommit::Reauthorize(prepared),
        ) => {
            if command.settings.is_some()
                || &prepared.account_id != account_id
                || &prepared.provider_kind != key.provider_kind()
            {
                return Err(invalid_receipt());
            }
            let prepared = admin_adapter::prepare_rotation(
                *prepared,
                None,
                context,
                gateway_admin::model::audit::MutationAuditOperation::ProviderAccountReauthorize,
            )?;
            let result = repository::rotate_provider_account_admin_in_transaction(
                transaction,
                prepared,
                true,
            )
            .await
            .map_err(|error| admin_store_error(ENTITY, error))?;
            admin_adapter::rotation_result(result, account_id.clone())?
        }
        _ => return Err(invalid_receipt()),
    };
    if let Some(operation) = managed_operation {
        if result.account_id.as_str() != operation.planned_account_id {
            return Err(super::super::oauth_operations::conflict());
        }
        if operation.is_new {
            sqlx::query("insert into managed_routes(route_id,account_id,proxy_id,generation,current_node,observation) values($1,$2,$3,1,$4,$5)")
                .bind(operation.operation_id).bind(&operation.planned_account_id).bind(&operation.proxy_id).bind(&operation.node).bind(&operation.observation)
                .execute(&mut **transaction).await.map_err(|e|unavailable_receipt().with_source(e))?;
            sqlx::query("update outbound_proxies set route_generation=1 where id=$1")
                .bind(&operation.proxy_id)
                .execute(&mut **transaction)
                .await
                .map_err(|e| unavailable_receipt().with_source(e))?;
            sqlx::query("update provider_accounts set managed_egress_required=true,outbound_proxy_id=$2,outbound_proxy_url=$3 where id=$1")
                .bind(&operation.planned_account_id).bind(&operation.proxy_id).bind(&operation.endpoint_url)
                .execute(&mut **transaction).await.map_err(|e|unavailable_receipt().with_source(e))?;
            let observation = operation
                .observation
                .as_ref()
                .filter(|value| value.success)
                .ok_or_else(invalid_receipt)?;
            sqlx::query("insert into managed_route_operations(operation_id,route_id,account_id,expected_generation,expected_source_revision,candidate,phase,observation,context,published_config_revision,published_proxy_url) values($1,$1,$2,0,$3,$4,'applied',$5,$6,$7,$8)")
                .bind(operation.operation_id).bind(&operation.planned_account_id).bind(operation.expected_source_revision).bind(&operation.node).bind(observation).bind(sqlx::types::Json(context))
                .bind(i64::try_from(result.config_revision.get()).map_err(|_|invalid_receipt())?).bind(&operation.endpoint_url)
                .execute(&mut **transaction).await.map_err(|e|unavailable_receipt().with_source(e))?;
            super::super::managed_routes::audit(
                transaction,
                context,
                gateway_admin::model::audit::MutationAuditOperation::ManagedRoutePublish,
                &operation.operation_id.to_string(),
                &["proxy_id", "generation", "current_node"],
                Some(store_revision(result.config_revision)?),
            )
            .await?;
        }
        sqlx::query("update managed_oauth_operations set phase='completed',receipt_flow_digest=$2,receipt_owner_digest=$3,reason=null,authorization_url=null,updated_at=now() where operation_id=$1 and phase='completing'")
            .bind(operation.operation_id).bind(key.flow_digest()).bind(key.owner_digest()).execute(&mut **transaction).await.map_err(|e|unavailable_receipt().with_source(e))?;
    }
    let accounts = [StoredAccount {
        account_id: result.account_id.to_string(),
        credential_revision: result.credential_revision.map(|revision| revision.get()),
    }];
    sqlx::query("insert into provider_authorization_receipts(provider_kind, flow_digest, owner_digest, config_revision, accounts_json) values ($1,$2,$3,$4,$5)")
            .bind(key.provider_kind().as_str()).bind(key.flow_digest()).bind(key.owner_digest())
            .bind(i64::try_from(result.config_revision.get()).map_err(|source| invalid_receipt().with_source(source))?)
            .bind(serde_json::to_value(accounts).map_err(|source| invalid_receipt().with_source(source))?)
            .execute(&mut **transaction).await.map_err(|source| unavailable_receipt().with_source(source))?;
    // 保留期长于 OAuth pending 上限；每次提交有界回收，避免引入第二套维护任务
    sqlx::query("delete from provider_authorization_receipts where ctid in (select ctid from provider_authorization_receipts where expires_at <= now() order by expires_at limit 128 for update skip locked)")
            .execute(&mut **transaction).await.map_err(|source| unavailable_receipt().with_source(source))?;
    Ok(AuthorizationCommitResult {
        result,
        newly_committed: true,
    })
}

async fn load_receipt<'a>(
    executor: impl sqlx::PgExecutor<'a>,
    key: &AuthorizationReceiptKey,
) -> AdminStoreResult<Option<CredentialMutationResult>> {
    let row: Option<(String, i64, serde_json::Value)> = sqlx::query_as("select owner_digest, config_revision, accounts_json from provider_authorization_receipts where provider_kind=$1 and flow_digest=$2 and expires_at>now()")
        .bind(key.provider_kind().as_str()).bind(key.flow_digest()).fetch_optional(executor).await.map_err(|source| unavailable_receipt().with_source(source))?;
    let Some((owner, revision, accounts)) = row else {
        return Ok(None);
    };
    if owner != key.owner_digest() {
        return Err(invalid_receipt());
    }
    let [account]: [StoredAccount; 1] = serde_json::from_value(accounts)
        .map_err(|source| unavailable_receipt().with_source(source))?;
    Ok(Some(CredentialMutationResult {
        config_revision: AdminRevision::new(
            u64::try_from(revision).map_err(|source| unavailable_receipt().with_source(source))?,
        )
        .map_err(|source| unavailable_receipt().with_source(source))?,
        account_id: CoreProviderAccountId::new(account.account_id)
            .map_err(|source| unavailable_receipt().with_source(source))?,
        credential_revision: account
            .credential_revision
            .map(|revision| {
                AdminRevision::new(revision)
                    .map_err(|source| unavailable_receipt().with_source(source))
            })
            .transpose()?,
    }))
}

fn invalid_receipt() -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::Invalid,
        ENTITY,
        "invalid authorization receipt binding",
    )
}
fn unavailable_receipt() -> AdminStoreError {
    AdminStoreError::new(
        AdminStoreErrorKind::Unavailable,
        ENTITY,
        "authorization receipt unavailable",
    )
}
