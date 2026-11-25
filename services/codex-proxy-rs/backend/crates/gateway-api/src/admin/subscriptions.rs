//! 订阅下载与解析预览的管理端 HTTP 边界
//!
//! 响应只带白名单字段：节点名、协议、可用标记、诊断码与计数
//! 主机、端口、凭据、UUID 与路径参数留在服务端

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use gateway_admin::model::subscriptions::{
    MAX_PREVIEW_RESPONSE_BYTES, SubscriptionPreview, SubscriptionPreviewError,
};
use serde::{Deserialize, Serialize};

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson, AdminQuery};
use gateway_admin::model::subscription_sources::{ApproveSource, SourceDetail};

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
    Router::new()
        .route("/api/admin/subscriptions/preview", post(preview::<S>))
        .route("/api/admin/subscriptions/sources", get(sources::<S>))
        .route(
            "/api/admin/subscriptions/sources/detail",
            get(source_detail::<S>),
        )
        .route(
            "/api/admin/subscriptions/sources/create",
            post(source_create::<S>),
        )
        .route(
            "/api/admin/subscriptions/sources/update",
            post(source_update::<S>),
        )
        .route(
            "/api/admin/subscriptions/sources/refresh",
            post(source_refresh::<S>),
        )
        .route(
            "/api/admin/subscriptions/sources/approve",
            post(source_approve::<S>),
        )
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateSource {
    name: String,
    source_url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceId {
    source_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshSource {
    source_id: String,
    expected_revision: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateSource {
    source_id: String,
    expected_revision: u64,
    name: String,
    source_url: Option<String>,
}

fn source_error(error: gateway_admin::model::AdminError) -> AdminError {
    use gateway_admin::model::AdminErrorKind;
    let reason = match error.kind() {
        AdminErrorKind::Conflict => "subscription_source_conflict",
        AdminErrorKind::NotFound => "subscription_source_not_found",
        AdminErrorKind::Invalid => "subscription_source_request_invalid",
        _ => "subscription_source_unavailable",
    };
    super::wire::map_admin_service_error(error).with_reason(reason)
}
fn source_response<T: Serialize>(value: T) -> Result<impl IntoResponse, AdminError> {
    let body = serde_json::to_vec(&AdminEnvelope::ok(value)).map_err(|_| AdminError::internal())?;
    if body.len() > MAX_PREVIEW_RESPONSE_BYTES {
        return Err(map_error(SubscriptionPreviewError::PreviewTooLarge));
    }
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    ))
}
async fn sources<S>(_: AdminAuth, State(state): State<S>) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let items = state
        .admin_services()
        .subscriptions()
        .sources()
        .await
        .map_err(source_error)?;
    source_response(serde_json::json!({"items":items}))
}
async fn source_detail<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminQuery(query): AdminQuery<SourceId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let detail = state
        .admin_services()
        .subscriptions()
        .source(&query.source_id)
        .await
        .map_err(source_error)?;
    source_response(detail)
}
async fn source_create<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<CreateSource>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let detail = state
        .admin_services()
        .subscriptions()
        .create_source(
            &body.name,
            &body.source_url,
            &auth.context().mutation_context(),
        )
        .await
        .map_err(source_error)?;
    source_response(detail)
}
async fn source_update<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<UpdateSource>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let detail = state
        .admin_services()
        .subscriptions()
        .update_source(
            &body.source_id,
            body.expected_revision,
            &body.name,
            body.source_url.as_deref(),
            &auth.context().mutation_context(),
        )
        .await
        .map_err(source_error)?;
    source_response(detail)
}
async fn source_refresh<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<RefreshSource>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let detail = state
        .admin_services()
        .subscriptions()
        .refresh_source(
            &body.source_id,
            body.expected_revision,
            &auth.context().mutation_context(),
        )
        .await
        .map_err(source_error)?;
    source_response(detail)
}
async fn source_approve<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<ApproveSource>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let detail: SourceDetail = state
        .admin_services()
        .subscriptions()
        .approve_source(body, &auth.context().mutation_context())
        .await
        .map_err(source_error)?;
    source_response(detail)
}
