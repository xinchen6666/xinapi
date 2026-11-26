//! 订阅来源的 PostgreSQL 读写、历史约束与配置审计事务

use std::{collections::BTreeMap, path::PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use gateway_admin::{
    model::{
        MutationContext,
        audit::MutationAuditOperation,
        subscription_sources::{SourceDocument, SourceSummary},
    },
    ports::{
        store::{AdminStoreError, AdminStoreErrorKind, AdminStoreResult},
        subscription_sources::SubscriptionStore,
    },
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::super::{append_admin_audit_event_in_transaction, bump_config_revision_in_transaction};
use super::encryption::{DocumentEncryption, invalid};
use crate::{
    ConflictKind, StoreError, StoreResult, admin_store_error, mutation_audit, postgres_unavailable,
};

const ENTITY: &str = "subscription source";
const MAX_SOURCES: i64 = 128;

pub struct PgSubscriptionStore {
    pool: PgPool,
    encryption: DocumentEncryption,
}

impl PgSubscriptionStore {
    pub fn new(pool: PgPool, key_path: impl Into<PathBuf>) -> StoreResult<Self> {
        let key_path = key_path.into();
        if key_path.as_os_str().is_empty() {
            return Err(invalid("订阅来源密钥路径需要配置"));
        }
        Ok(Self {
            pool,
            encryption: DocumentEncryption::new(key_path),
        })
    }

    async fn decode(&self, row: SourceRow) -> StoreResult<SourceDocument> {
        let document = self.encryption.decrypt(row.id, &row.document).await?;
        if Uuid::parse_str(&document.summary.id).ok() != Some(row.id)
            || i64::try_from(document.summary.revision).ok() != Some(row.revision)
            || document.summary.name != row.name
            || document.summary.updated_at.timestamp_micros() != row.updated_at.timestamp_micros()
        {
            return Err(invalid("订阅来源文档与数据库元数据校验失败"));
        }
        Ok(document)
    }

    async fn save_document(
        &self,
        mut document: SourceDocument,
        expected: Option<u64>,
        operation: &'static str,
        context: &MutationContext,
    ) -> StoreResult<()> {
        // 文档和索引共用 PostgreSQL 微秒精度，确保认证文档与列值一致
        document.summary.updated_at =
            DateTime::from_timestamp_micros(document.summary.updated_at.timestamp_micros())
                .ok_or_else(|| invalid("订阅来源更新时间超出数据库范围"))?;
        let id = source_id(&document.summary.id)?;
        let (audit_operation, changes_config) = operation_kind(operation)?;
        if (expected.is_none())
            != (audit_operation == MutationAuditOperation::SubscriptionSourceCreate)
        {
            return Err(invalid("订阅来源操作需要匹配创建或更新版本"));
        }
        let revision = expected
            .unwrap_or(0)
            .checked_add(1)
            .filter(|next| *next == document.summary.revision)
            .and_then(|next| i64::try_from(next).ok())
            .ok_or_else(|| invalid("订阅来源文档版本需要等于预期版本加一"))?;
        // 已有来源时密钥属于既有部署；创建新来源也只接受原密钥
        let create_key = if expected.is_none() {
            let existing = sqlx::query_as::<_, SourceRow>(
                "select id, revision, name, document, updated_at from subscription_sources order by id limit 1",
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(|source| postgres_unavailable("inspect subscription key ownership", source))?;
            if let Some(existing) = existing {
                self.decode(existing).await?;
                false
            } else {
                true
            }
        } else {
            false
        };
        let encrypted = self.encryption.encrypt(id, &document, create_key).await?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|source| postgres_unavailable("begin subscription mutation", source))?;
        let result = self
            .save_in_transaction(
                &mut transaction,
                id,
                &document,
                &encrypted,
                expected,
                revision,
                audit_operation,
                changes_config,
                context,
            )
            .await;
        match result {
            Ok(()) => transaction
                .commit()
                .await
                .map_err(|source| postgres_unavailable("commit subscription mutation", source)),
            Err(error) => match transaction.rollback().await {
                Ok(()) => Err(error),
                Err(cleanup) => Err(error.with_cleanup(cleanup)),
            },
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "来源 CAS、配置修订与审计共同持有同一事务上下文"
    )]
    async fn save_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        id: Uuid,
        document: &SourceDocument,
        encrypted: &[u8],
        expected: Option<u64>,
        revision: i64,
        operation: MutationAuditOperation,
        changes_config: bool,
        context: &MutationContext,
    ) -> StoreResult<()> {
        // 配置行先于来源行加锁，创建数量和运行配置变更沿用统一事务顺序
        if changes_config {
            sqlx::query_scalar::<_, i64>(
                "select config_revision from runtime_settings where id = 1 for update",
            )
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|source| postgres_unavailable("lock subscription config revision", source))?
            .ok_or_else(|| StoreError::NotFound {
                entity: "runtime settings",
                id: "1".to_owned(),
                source: None,
            })?;
        }
        let previous = sqlx::query_as::<_, SourceRow>("select id, revision, name, document, updated_at from subscription_sources where id = $1 for update")
            .bind(id).fetch_optional(&mut **transaction).await
            .map_err(|source| postgres_unavailable("lock subscription source", source))?;
        match (expected, previous) {
            (None, None) => {
                let count =
                    sqlx::query_scalar::<_, i64>("select count(*) from subscription_sources")
                        .fetch_one(&mut **transaction)
                        .await
                        .map_err(|source| {
                            postgres_unavailable("count subscription sources", source)
                        })?;
                if count >= MAX_SOURCES {
                    return Err(invalid("订阅来源数量上限为 128 项"));
                }
                validate_histories(None, document)?;
                sqlx::query("insert into subscription_sources (id, revision, name, document, updated_at) values ($1, $2, $3, $4, $5)")
                    .bind(id).bind(revision).bind(&document.summary.name).bind(encrypted).bind(document.summary.updated_at)
                    .execute(&mut **transaction).await
                    .map_err(|source| postgres_unavailable("create subscription source", source))?;
            }
            (Some(expected), Some(previous))
                if u64::try_from(previous.revision).ok() == Some(expected) =>
            {
                let previous = self.decode(previous).await?;
                validate_histories(Some(&previous), document)?;
                let changed = sqlx::query("update subscription_sources set revision = $2, name = $3, document = $4, updated_at = $5 where id = $1 and revision = $6")
                    .bind(id).bind(revision).bind(&document.summary.name).bind(encrypted).bind(document.summary.updated_at)
                    .bind(i64::try_from(expected).map_err(|_| invalid("订阅来源版本超出数据库范围"))?)
                    .execute(&mut **transaction).await
                    .map_err(|source| postgres_unavailable("update subscription source", source))?;
                if changed.rows_affected() != 1 {
                    return Err(stale(id));
                }
            }
            _ => return Err(stale(id)),
        }
        let config_revision = if changes_config {
            Some(bump_config_revision_in_transaction(transaction).await?)
        } else {
            None
        };
        append_admin_audit_event_in_transaction(
            transaction,
            mutation_audit(
                context,
                operation,
                &id.to_string(),
                vec!["document".to_owned(), "revision".to_owned()],
            ),
            config_revision,
        )
        .await
    }
}

#[async_trait]
impl SubscriptionStore for PgSubscriptionStore {
    async fn list(&self) -> AdminStoreResult<Vec<SourceSummary>> {
        let mut rows = sqlx::query_as::<_, SourceRow>("select id, revision, name, document, updated_at from subscription_sources order by name, id")
            .fetch(&self.pool);
        let mut summaries = Vec::new();
        while let Some(row) = rows.try_next().await.map_err(|source| {
            store_error(postgres_unavailable("list subscription sources", source))
        })? {
            summaries.push(self.decode(row).await.map_err(store_error)?.summary);
        }
        Ok(summaries)
    }

    async fn load(&self, id: &str) -> AdminStoreResult<SourceDocument> {
        let uuid = source_id(id).map_err(store_error)?;
        let row = sqlx::query_as::<_, SourceRow>("select id, revision, name, document, updated_at from subscription_sources where id = $1")
            .bind(uuid).fetch_optional(&self.pool).await
            .map_err(|source| store_error(postgres_unavailable("load subscription source", source)))?
            .ok_or_else(|| store_error(StoreError::NotFound { entity: ENTITY, id: id.to_owned(), source: None }))?;
        self.decode(row).await.map_err(store_error)
    }

    async fn save(
        &self,
        document: SourceDocument,
        expected: Option<u64>,
        operation: &'static str,
        context: &MutationContext,
    ) -> AdminStoreResult<()> {
        self.save_document(document, expected, operation, context)
            .await
            .map_err(store_error)
    }
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    id: Uuid,
    revision: i64,
    name: String,
    document: Vec<u8>,
    updated_at: DateTime<Utc>,
}

fn validate_histories(
    previous: Option<&SourceDocument>,
    document: &SourceDocument,
) -> StoreResult<()> {
    let mut versions = BTreeMap::new();
    for version in &document.versions {
        if versions.insert(&version.id, version).is_some() {
            return Err(invalid("订阅来源连接版本 ID 需要唯一"));
        }
    }
    let mut approved_sets = BTreeMap::new();
    for approved in &document.approved_sets {
        if approved_sets.insert(&approved.id, approved).is_some() {
            return Err(invalid("订阅来源批准集合 ID 需要唯一"));
        }
    }
    if let Some(previous) = previous {
        if previous
            .versions
            .iter()
            .any(|version| versions.get(&version.id).copied() != Some(version))
        {
            return Err(invalid("订阅来源历史连接版本需要完整保留原值"));
        }
        if previous
            .approved_sets
            .iter()
            .any(|approved| approved_sets.get(&approved.id).copied() != Some(approved))
        {
            return Err(invalid("订阅来源历史批准集合需要完整保留原值"));
        }
    }
    Ok(())
}

fn operation_kind(operation: &str) -> StoreResult<(MutationAuditOperation, bool)> {
    Ok(match operation {
        "create" => (MutationAuditOperation::SubscriptionSourceCreate, true),
        "update" => (MutationAuditOperation::SubscriptionSourceUpdate, true),
        "review" => (MutationAuditOperation::SubscriptionSourceReview, false),
        "approve" => (MutationAuditOperation::SubscriptionSourceApprove, true),
        "refresh_failed" => (
            MutationAuditOperation::SubscriptionSourceRefreshFailed,
            false,
        ),
        _ => return Err(invalid("订阅来源操作类型需要使用约定值")),
    })
}

fn source_id(id: &str) -> StoreResult<Uuid> {
    Uuid::parse_str(id).map_err(|_| invalid("订阅来源 ID 需要使用 UUID"))
}

fn stale(id: Uuid) -> StoreError {
    StoreError::Conflict {
        entity: ENTITY,
        id: id.to_string(),
        kind: ConflictKind::StaleRevision,
        source: None,
    }
}

fn store_error(error: StoreError) -> AdminStoreError {
    if let StoreError::InvalidData { message, .. } = &error {
        AdminStoreError::new(AdminStoreErrorKind::Invalid, ENTITY, message.clone())
            .with_source(error)
    } else {
        admin_store_error(ENTITY, error)
    }
}
