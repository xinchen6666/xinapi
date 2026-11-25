//! 订阅来源的窄持久化合同与原子版本检查

use crate::{
    model::{
        MutationContext,
        subscription_sources::{SourceDocument, SourceSummary},
    },
    ports::store::AdminStoreResult,
};
use async_trait::async_trait;

#[async_trait]
pub trait SubscriptionStore: Send + Sync {
    async fn list(&self) -> AdminStoreResult<Vec<SourceSummary>>;
    async fn load(&self, id: &str) -> AdminStoreResult<SourceDocument>;
    /// expected=None 创建，Some 只接受数据库当前来源版本，整个文档在同一事务替换
    async fn save(
        &self,
        document: SourceDocument,
        expected: Option<u64>,
        operation: &'static str,
        context: &MutationContext,
    ) -> AdminStoreResult<()>;
}
