//! 受管引擎进程的窄端口；Host 实现启动、停止与观测

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::model::{
    engines::{EngineCapacity, EngineFacts, EngineServiceError, EngineSnapshot},
    managed_routes::RouteObservation,
};

use super::proxy::ProxyProbe;

#[async_trait]
pub trait EngineHost: Send + Sync {
    /// 启动入口并在就绪探测通过后返回观测；同一入口重复启动复用运行中的进程
    async fn start(&self, facts: EngineFacts) -> Result<EngineSnapshot, EngineServiceError>;
    /// 停止入口进程并等待回收；入口记录保留以便再次启动
    async fn stop(&self, entry_id: &str) -> Result<EngineSnapshot, EngineServiceError>;
    /// 列出当前已知入口及其最新观测
    async fn list(&self) -> Result<Vec<EngineSnapshot>, EngineServiceError>;
    /// 槽位和入口观测使用同一 Host 状态
    async fn inventory(&self) -> Result<(Vec<EngineSnapshot>, EngineCapacity), EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 临时入口先结束进程与私有文件，再释放占用的槽位
    async fn test_node<'a>(
        &self,
        _facts: EngineFacts,
        _probe: &dyn ProxyProbe,
        _validate: BoxFuture<'a, Result<(), EngineServiceError>>,
    ) -> Result<RouteObservation, EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 确认绑定事实后停止账号入口，与 Host 切换共用入口操作锁
    async fn stop_account(
        &self,
        _entry_id: &str,
        _node: &crate::model::managed_routes::RouteNode,
    ) -> Result<EngineSnapshot, EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 服务端私有入口地址，只供受管路由发布给代理仓储
    async fn endpoint(
        &self,
        _entry_id: &str,
    ) -> Result<gateway_core::account::OutboundProxy, EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 发布批准版本，保留原入口与凭据并显式终止旧连接
    async fn replace(&self, _facts: EngineFacts) -> Result<EngineSnapshot, EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 授权服务确认零账号引用后，回收预留的未来账号入口
    async fn discard_authorization(&self, _entry_id: &str) -> Result<(), EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
    /// 候选验证结束后回收其临时入口，账号入口继续保留
    async fn discard_candidate(&self, _entry_id: &str) -> Result<(), EngineServiceError> {
        Err(EngineServiceError::Unavailable)
    }
}
