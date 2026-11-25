//! 受管引擎归属与生命周期操作的管理端 HTTP 边界
//!
//! 身份和公开观测经固定封套交付，入口认证材料留在 Host

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use gateway_admin::model::{
    AdminErrorKind,
    engines::{EngineOwner, EngineServiceError, EngineView},
};
use serde::{Deserialize, Serialize};

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartEngine {
    owner: EngineOwner,
    source_id: String,
    node_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StopEngine {
    entry_id: String,
}

#[derive(Serialize)]
struct EngineListView {
    items: Vec<EngineView>,
}

#[derive(Serialize)]
struct EngineViewResponse {
    engine: EngineView,
}

pub fn router<S>() -> Router<S>
where
    S: crate::auth::SessionState + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/admin/engines", get(engines::<S>))
        .route("/api/admin/engines/start", post(engine_start::<S>))
        .route("/api/admin/engines/stop", post(engine_stop::<S>))
}

fn engine_error(error: EngineServiceError) -> AdminError {
    let (status, kind, message) = match error {
        EngineServiceError::BinaryMissing => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎程序缺失，请核对部署镜像",
        ),
        EngineServiceError::ArtifactIdentity => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎制品身份校验失败，请核对部署文件",
        ),
        EngineServiceError::Capacity => (
            StatusCode::TOO_MANY_REQUESTS,
            AdminErrorKind::RateLimited,
            "引擎实例或登记数量达到上限",
        ),
        EngineServiceError::PortConflict => (
            StatusCode::CONFLICT,
            AdminErrorKind::Conflict,
            "入口端口占用中，请释放端口后重试",
        ),
        EngineServiceError::StartTimeout => (
            StatusCode::GATEWAY_TIMEOUT,
            AdminErrorKind::BadGateway,
            "引擎启动超时",
        ),
        EngineServiceError::Cancelled => (
            StatusCode::CONFLICT,
            AdminErrorKind::Conflict,
            "引擎操作已取消",
        ),
        EngineServiceError::ShuttingDown => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎宿主正在关闭",
        ),
        EngineServiceError::BindingConflict => (
            StatusCode::CONFLICT,
            AdminErrorKind::Conflict,
            "入口已绑定其他来源、节点或版本，请核对原绑定",
        ),
        EngineServiceError::UnknownEntry => (
            StatusCode::NOT_FOUND,
            AdminErrorKind::NotFound,
            "引擎入口记录缺失",
        ),
        EngineServiceError::OwnerInvalid => (
            StatusCode::UNPROCESSABLE_ENTITY,
            AdminErrorKind::Invalid,
            "入口归属标识校验失败，请使用有效账号 ID 或预留项 UUID",
        ),
        EngineServiceError::Registry => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎登记读写失败，请核对持久目录与登记文件",
        ),
        EngineServiceError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎服务异常",
        ),
        EngineServiceError::SourceMissing => (
            StatusCode::NOT_FOUND,
            AdminErrorKind::NotFound,
            "订阅来源记录缺失",
        ),
        EngineServiceError::NodeMissing => (
            StatusCode::NOT_FOUND,
            AdminErrorKind::NotFound,
            "订阅节点记录缺失",
        ),
        EngineServiceError::NodeInactive => (
            StatusCode::CONFLICT,
            AdminErrorKind::Conflict,
            "该节点当前停用，请核对来源批准状态",
        ),
        EngineServiceError::NodeUnsupported => (
            StatusCode::UNPROCESSABLE_ENTITY,
            AdminErrorKind::Invalid,
            "引擎适配器尚待实现此节点协议或连接选项",
        ),
        EngineServiceError::ProcessFailed => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::BadGateway,
            "引擎进程启动或退出异常",
        ),
        EngineServiceError::Store => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "来源或账号存储读取失败",
        ),
        EngineServiceError::Configuration => (
            StatusCode::SERVICE_UNAVAILABLE,
            AdminErrorKind::Unavailable,
            "引擎部署配置校验失败",
        ),
    };
    super::wire::map_admin_service_error(gateway_admin::model::AdminError::new(kind, message))
        .with_reason(error.reason())
        .with_status(status)
}

fn engine_response<T: Serialize>(value: T) -> Result<impl IntoResponse, AdminError> {
    let body = serde_json::to_vec(&AdminEnvelope::ok(value)).map_err(|_| AdminError::internal())?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body,
    ))
}

async fn engines<S>(_: AdminAuth, State(state): State<S>) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let items = state
        .admin_services()
        .engines()
        .list()
        .await
        .map_err(engine_error)?;
    engine_response(EngineListView { items })
}

async fn engine_start<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<StartEngine>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let engine = state
        .admin_services()
        .engines()
        .start(body.owner, &body.source_id, &body.node_id)
        .await
        .map_err(engine_error)?;
    engine_response(EngineViewResponse { engine })
}

async fn engine_stop<S>(
    _: AdminAuth,
    State(state): State<S>,
    AdminJson(body): AdminJson<StopEngine>,
) -> Result<impl IntoResponse, AdminError>
where
    S: crate::auth::SessionState + Send + Sync,
{
    let engine = state
        .admin_services()
        .engines()
        .stop(&body.entry_id)
        .await
        .map_err(engine_error)?;
    engine_response(EngineViewResponse { engine })
}
