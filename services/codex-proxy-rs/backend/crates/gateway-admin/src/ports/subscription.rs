//! 订阅文档下载端口与组合根提供的下载能力

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

use crate::model::subscriptions::SubscriptionFetchPolicy;

/// 一次受控下载的结果；body 是解压后的正文
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedDocument {
    pub body: Vec<u8>,
}

/// 下载失败分类，由管理用例映射成脱敏错误码
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionFetchError {
    #[error("subscription target scheme or host is rejected")]
    TargetRejected,
    #[error("subscription body exceeds the size limit")]
    TooLarge,
    #[error("subscription download exceeded the budget")]
    Timeout,
    #[error("subscription download exceeded the redirect limit")]
    TooManyRedirects,
    #[error("subscription download failed")]
    Failed,
}

/// 订阅文档下载端口
///
/// 实现方负责逐跳复核目标、限制正文与解压规模、遵守传入的时间预算，
/// 并把任何上游细节收敛为 [`SubscriptionFetchError`]
#[async_trait]
pub trait SubscriptionDocumentSource: Send + Sync {
    /// 在 `budget` 内下载一个订阅文档
    ///
    /// # Errors
    ///
    /// 目标被策略拒绝、正文超限、预算耗尽、跳转过多或传输失败时返回对应分类
    async fn fetch(
        &self,
        url: &str,
        policy: &SubscriptionFetchPolicy,
        budget: Duration,
    ) -> Result<FetchedDocument, SubscriptionFetchError>;
}

/// 组合根提供的订阅下载运行能力
pub struct SubscriptionRuntime {
    pub source: Arc<dyn SubscriptionDocumentSource>,
    pub policy: SubscriptionFetchPolicy,
}
