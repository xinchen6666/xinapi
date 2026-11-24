//! 订阅预览用例：受控下载、格式判定、逐节点结果与 HTTP 型 provider 物化
//!
//! 本步只做一次性预览：下载与解析结果直接返回，状态写入留给后续任务

mod parse;

use std::{sync::Arc, time::Instant};

use async_trait::async_trait;

use crate::{
    model::subscriptions::{
        DiagnosticCode, FETCH_BUDGET, NodeCandidate, PreviewDiagnostic, SubscriptionFetchPolicy,
        SubscriptionPreview, SubscriptionPreviewError, SubscriptionSourceError,
    },
    ports::subscription::{FetchedDocument, SubscriptionDocumentSource, SubscriptionFetchError},
};

use parse::{ProviderReference, parse_document};

const PROVIDER_DOWNLOAD_FAILED_MESSAGE: &str = "The proxy provider document download failed.";
const PROVIDER_UNREADABLE_MESSAGE: &str = "The proxy provider document is unreadable.";
const PROVIDER_DEPTH_LIMIT_MESSAGE: &str = "Nested proxy providers stay one level deep.";

/// 一次性订阅预览
#[async_trait]
pub trait SubscriptionsService: Send + Sync {
    /// 下载并解析订阅源，返回可公开的节点事实与诊断
    ///
    /// # Errors
    ///
    /// 源地址不合法、目标被策略拒绝、下载失败或文档整体不可读时返回稳定分类
    async fn preview(
        &self,
        source_url: &str,
    ) -> Result<SubscriptionPreview, SubscriptionPreviewError>;
}

pub(crate) struct DefaultSubscriptionsService {
    source: Arc<dyn SubscriptionDocumentSource>,
    policy: SubscriptionFetchPolicy,
}

impl DefaultSubscriptionsService {
    pub(crate) fn new(
        source: Arc<dyn SubscriptionDocumentSource>,
        policy: SubscriptionFetchPolicy,
    ) -> Self {
        Self { source, policy }
    }

    /// 整次预览共用一个时限，provider 拉取计入同一预算
    async fn fetch_document(
        &self,
        url: &str,
        deadline: Instant,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        let budget = deadline.saturating_duration_since(Instant::now());
        if budget.is_zero() {
            return Err(SubscriptionFetchError::Timeout);
        }
        self.source.fetch(url, &self.policy, budget).await
    }

    /// 把 provider 文档的节点并入候选；单个 provider 失败只记诊断
    async fn extend_providers(
        &self,
        providers: &[ProviderReference],
        deadline: Instant,
        candidates: &mut Vec<NodeCandidate>,
        diagnostics: &mut Vec<PreviewDiagnostic>,
    ) {
        for provider in providers {
            let fetched = match self.policy.resolve_source(&provider.url) {
                Ok(url) => self.fetch_document(url.as_str(), deadline).await,
                Err(_) => Err(SubscriptionFetchError::TargetRejected),
            };
            let document = match fetched {
                Ok(document) => document,
                Err(_) => {
                    diagnostics.push(PreviewDiagnostic::new(
                        DiagnosticCode::ProviderFetchFailed,
                        PROVIDER_DOWNLOAD_FAILED_MESSAGE,
                        provider.name.clone(),
                    ));
                    continue;
                }
            };
            let parsed = match parse_document(&document.body) {
                Ok(parsed) => parsed,
                Err(_) => {
                    diagnostics.push(PreviewDiagnostic::new(
                        DiagnosticCode::ProviderFetchFailed,
                        PROVIDER_UNREADABLE_MESSAGE,
                        provider.name.clone(),
                    ));
                    continue;
                }
            };
            candidates.extend(parsed.candidates);
            diagnostics.extend(parsed.diagnostics);
            // provider 深度上限为一级，provider 文档内的 provider 只记诊断
            diagnostics.extend(parsed.providers.into_iter().map(|nested| {
                PreviewDiagnostic::new(
                    DiagnosticCode::ProviderDepthLimit,
                    PROVIDER_DEPTH_LIMIT_MESSAGE,
                    nested.name,
                )
            }));
        }
    }
}

#[async_trait]
impl SubscriptionsService for DefaultSubscriptionsService {
    async fn preview(
        &self,
        source_url: &str,
    ) -> Result<SubscriptionPreview, SubscriptionPreviewError> {
        let url = self
            .policy
            .resolve_source(source_url)
            .map_err(source_error)?;
        let deadline = Instant::now() + FETCH_BUDGET;
        let document = self
            .fetch_document(url.as_str(), deadline)
            .await
            .map_err(fetch_error)?;
        let parsed =
            parse_document(&document.body).map_err(|_| SubscriptionPreviewError::Unreadable)?;
        let mut candidates = parsed.candidates;
        let mut diagnostics = parsed.diagnostics;
        self.extend_providers(
            &parsed.providers,
            deadline,
            &mut candidates,
            &mut diagnostics,
        )
        .await;
        let accepted = candidates
            .iter()
            .filter(|candidate| candidate.supported)
            .count();
        Ok(SubscriptionPreview {
            source_url: source_url.to_owned(),
            format: parsed.format,
            fetched_bytes: document.body.len(),
            accepted,
            rejected: candidates.len() - accepted,
            candidates,
            diagnostics,
        })
    }
}

fn source_error(error: SubscriptionSourceError) -> SubscriptionPreviewError {
    match error {
        SubscriptionSourceError::Malformed => SubscriptionPreviewError::SourceInvalid,
        SubscriptionSourceError::TargetRejected => SubscriptionPreviewError::TargetRejected,
    }
}

fn fetch_error(error: SubscriptionFetchError) -> SubscriptionPreviewError {
    match error {
        SubscriptionFetchError::TargetRejected => SubscriptionPreviewError::TargetRejected,
        SubscriptionFetchError::TooLarge => SubscriptionPreviewError::TooLarge,
        SubscriptionFetchError::Timeout => SubscriptionPreviewError::Timeout,
        SubscriptionFetchError::TooManyRedirects => SubscriptionPreviewError::TooManyRedirects,
        SubscriptionFetchError::Failed => SubscriptionPreviewError::FetchFailed,
    }
}
