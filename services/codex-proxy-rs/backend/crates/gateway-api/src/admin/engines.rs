//! 受管引擎入口的管理端 HTTP 边界
//!
//! 只发布入口地址、进程状态与资源观测；连接内容留在服务端配置目录

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use gateway_admin::model::{
    AdminErrorKind,
    engines::{EngineServiceError, EngineView},
};
use serde::{Deserialize, Serialize};

use super::{AdminAuth, AdminEnvelope, AdminError, AdminJson};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartEngine {
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
    let status = match error {
        EngineServiceError::BinaryMissing
        | EngineServiceError::Unavailable
        | EngineServiceError::ProcessFailed
        | EngineServiceError::Store => StatusCode::SERVICE_UNAVAILABLE,
        EngineServiceError::StartTimeout => StatusCode::GATEWAY_TIMEOUT,
        EngineServiceError::Capacity => StatusCode::TOO_MANY_REQUESTS,
        EngineServiceError::PortConflict | EngineServiceError::NodeInactive => StatusCode::CONFLICT,
        EngineServiceError::NodeUnsupported => StatusCode::UNPROCESSABLE_ENTITY,
        EngineServiceError::SourceMissing
        | EngineServiceError::NodeMissing
        | EngineServiceError::UnknownEntry => StatusCode::NOT_FOUND,
    };
    let message = match error {
        EngineServiceError::BinaryMissing => "引擎程序缺失，请核对部署镜像",
        EngineServiceError::Capacity => "引擎实例达到上限，请先停用空闲入口",
        EngineServiceError::PortConflict => "入口端口占用中，请释放端口后重试",
        EngineServiceError::StartTimeout => "引擎启动超时，请查看引擎日志",
        EngineServiceError::UnknownEntry => "引擎入口记录缺失",
        EngineServiceError::Unavailable => "引擎服务异常",
        EngineServiceError::SourceMissing => "订阅来源记录缺失",
        EngineServiceError::NodeMissing => "订阅节点记录缺失",
        EngineServiceError::NodeInactive => "该节点当前停用，请刷新来源后重试",
        EngineServiceError::NodeUnsupported => "该节点协议仅支持 HTTP 与 SOCKS5 引擎入口",
        EngineServiceError::ProcessFailed => "引擎进程退出，请查看引擎日志",
        EngineServiceError::Store => "来源存储读取失败",
    };
    let kind = match error {
        EngineServiceError::SourceMissing
        | EngineServiceError::NodeMissing
        | EngineServiceError::UnknownEntry => AdminErrorKind::NotFound,
        EngineServiceError::PortConflict | EngineServiceError::NodeInactive => {
            AdminErrorKind::Conflict
        }
        EngineServiceError::NodeUnsupported => AdminErrorKind::Invalid,
        EngineServiceError::Capacity => AdminErrorKind::RateLimited,
        EngineServiceError::StartTimeout | EngineServiceError::ProcessFailed => {
            AdminErrorKind::BadGateway
        }
        EngineServiceError::BinaryMissing
        | EngineServiceError::Unavailable
        | EngineServiceError::Store => AdminErrorKind::Unavailable,
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
        .start(&body.source_id, &body.node_id)
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
