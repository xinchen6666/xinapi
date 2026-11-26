//! 账号受管路由候选、批准与操作读回的HTTP边界

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson, AdminQuery};
use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use gateway_admin::model::managed_routes::PrepareRoute;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationId {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountId {
    account_id: String,
}

pub(super) fn router<S>() -> Router<S>
where
    S: crate::auth::SessionState + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/admin/managed-routes", get(route::<S>))
        .route("/api/admin/managed-routes/operations", get(operation::<S>))
        .route("/api/admin/managed-routes/prepare", post(prepare::<S>))
        .route("/api/admin/managed-routes/approve", post(approve::<S>))
        .route("/api/admin/managed-routes/reconcile", post(reconcile::<S>))
        .route("/api/admin/managed-routes/abort", post(abort::<S>))
}
fn error(error: gateway_admin::model::AdminError) -> AdminError {
    use gateway_admin::model::AdminErrorKind;
    let reason = match error.kind() {
        AdminErrorKind::Conflict => "managed_route_conflict",
        AdminErrorKind::NotFound => "managed_route_missing",
        AdminErrorKind::Invalid => "managed_route_invalid",
        AdminErrorKind::RateLimited => "managed_route_capacity",
        AdminErrorKind::BadGateway => "managed_route_candidate_failed",
        _ => "managed_route_unavailable",
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
async fn abort<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminJson(command): AdminJson<OperationId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .managed_routes()
            .abort(&command.operation_id)
            .await
            .map_err(error)?,
    )
}

async fn prepare<S>(
    auth: AdminAuth,
    State(state): State<S>,
    AdminJson(command): AdminJson<PrepareRoute>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .managed_routes()
            .prepare(command, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
async fn approve<S>(
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
            .managed_routes()
            .approve(&command.operation_id, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
async fn reconcile<S>(
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
            .managed_routes()
            .reconcile(&command.operation_id, &auth.context().mutation_context())
            .await
            .map_err(error)?,
    )
}
async fn operation<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminQuery(query): AdminQuery<OperationId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .managed_routes()
            .operation(&query.operation_id)
            .await
            .map_err(error)?,
    )
}
async fn route<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminQuery(query): AdminQuery<AccountId>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    response(
        state
            .admin_services()
            .managed_routes()
            .route(&query.account_id)
            .await
            .map_err(error)?,
    )
}
