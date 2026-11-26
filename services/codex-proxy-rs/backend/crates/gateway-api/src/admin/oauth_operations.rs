//! 管理员专用OAuth操作与只读完成回执的HTTP边界

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson, AdminQuery};
use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use gateway_admin::model::oauth_operations::StartOAuthOperation;
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationId {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Complete {
    operation_id: String,
    callback_url: String,
}
pub(super) fn router<S>() -> Router<S>
where
    S: crate::auth::SessionState + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/admin/oauth-operations", get(read::<S>))
        .route("/api/admin/oauth-operations/start", post(start::<S>))
        .route("/api/admin/oauth-operations/complete", post(complete::<S>))
        .route("/api/admin/oauth-operations/cancel", post(cancel::<S>))
}
pub(super) fn error(error: gateway_admin::model::AdminError) -> AdminError {
    use gateway_admin::model::AdminErrorKind;
    let reason = match error.kind() {
        AdminErrorKind::Conflict => "oauth_operation_conflict",
        AdminErrorKind::Invalid => "oauth_operation_invalid",
        AdminErrorKind::NotFound => "oauth_operation_missing",
        AdminErrorKind::BadGateway => "oauth_operation_egress_failed",
        _ => "oauth_operation_unavailable",
    };
    super::wire::map_admin_service_error(error).with_reason(reason)
}
fn response<T: Serialize>(value: T) -> Result<impl IntoResponse, AdminError> {
    let body = serde_json::to_vec(&AdminEnvelope::ok(value)).map_err(|_| AdminError::internal())?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    ))
}
async fn read<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminQuery(query): AdminQuery<OperationId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .credentials()
            .oauth_operation(&query.operation_id, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
async fn start<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(command): AdminJson<StartOAuthOperation>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .credentials()
            .start_oauth_operation(command, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
async fn complete<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(command): AdminJson<Complete>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .credentials()
            .complete_oauth_operation(
                &command.operation_id,
                command.callback_url,
                &auth.context().mutation_context(),
            )
            .await
            .map_err(error)?,
    )
}
async fn cancel<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(command): AdminJson<OperationId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .credentials()
            .cancel_oauth_operation(&command.operation_id, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
