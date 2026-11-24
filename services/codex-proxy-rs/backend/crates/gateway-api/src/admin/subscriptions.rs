//! 订阅下载与解析预览的管理端 HTTP 边界
//!
//! 响应只带白名单字段：节点名、协议、可用标记、诊断码与计数
//! 主机、端口、凭据、UUID 与路径参数留在服务端

use axum::{
    Router, extract::State, http::StatusCode, http::header, response::IntoResponse, routing::post,
};
use gateway_admin::model::subscriptions::{
    MAX_PREVIEW_RESPONSE_BYTES, SubscriptionPreview, SubscriptionPreviewError,
};
use serde::{Deserialize, Serialize};

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson};

/// 正文内容类失败按 422 返回：请求本身合法，文档内容无法使用
fn content_unusable(error: SubscriptionPreviewError) -> bool {
    matches!(
        error,
        SubscriptionPreviewError::Unreadable
            | SubscriptionPreviewError::TooLarge
            | SubscriptionPreviewError::PreviewTooLarge
    )
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviewRequest {
    source_url: String,
}

#[derive(Serialize)]
struct CandidateView {
    name: String,
    protocol: &'static str,
    parse_valid: bool,
    supported: bool,
}

#[derive(Serialize)]
struct DiagnosticView {
    code: &'static str,
    message: String,
    subject: String,
}

/// 预览结果按已确认的 snake_case 字段名发布，协议与格式使用稳定枚举值
#[derive(Serialize)]
struct PreviewView {
    source_url: String,
    format: &'static str,
    fetched_bytes: u64,
    accepted: usize,
    rejected: usize,
    candidates: Vec<CandidateView>,
    diagnostics: Vec<DiagnosticView>,
}

impl From<SubscriptionPreview> for PreviewView {
    fn from(preview: SubscriptionPreview) -> Self {
        Self {
            source_url: preview.source_url,
            format: preview.format.as_str(),
            fetched_bytes: u64::try_from(preview.fetched_bytes).unwrap_or(u64::MAX),
            accepted: preview.accepted,
            rejected: preview.rejected,
            candidates: preview
                .candidates
                .into_iter()
                .map(|candidate| CandidateView {
                    name: candidate.name,
                    protocol: candidate.protocol.as_str(),
                    parse_valid: candidate.parse_valid,
                    supported: candidate.supported,
                })
                .collect(),
            diagnostics: preview
                .diagnostics
                .into_iter()
                .map(|diagnostic| DiagnosticView {
                    code: diagnostic.code.as_str(),
                    message: diagnostic.message,
                    subject: diagnostic.subject,
                })
                .collect(),
        }
    }
}

pub fn router<S>() -> Router<S>
where
    S: crate::auth::SessionState + Clone + Send + Sync + 'static,
{
    Router::new().route("/api/admin/subscriptions/preview", post(preview::<S>))
}

fn map_error(error: SubscriptionPreviewError) -> AdminError {
    let reason = error.reason();
    let response =
        super::wire::map_admin_service_error(error.into_admin_error()).with_reason(reason);
    if content_unusable(error) {
        return response.with_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    response
}

async fn preview<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminJson(request): AdminJson<PreviewRequest>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let preview = state
        .admin_services()
        .subscriptions()
        .preview(&request.source_url)
        .await
        .map_err(map_error)?;
    let body = serde_json::to_vec(&AdminEnvelope::ok(PreviewView::from(preview)))
        .map_err(|_| map_error(SubscriptionPreviewError::PreviewTooLarge))?;
    if body.len() > MAX_PREVIEW_RESPONSE_BYTES {
        return Err(map_error(SubscriptionPreviewError::PreviewTooLarge));
    }
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    ))
}
