//! 受管引擎进程的窄端口；Host 实现启动、停止与观测

use async_trait::async_trait;

use crate::model::engines::{EngineFacts, EngineServiceError, EngineSnapshot};

#[async_trait]
pub trait EngineHost: Send + Sync {
    /// 启动入口并在就绪探测通过后返回观测；同一入口重复启动复用运行中的进程
    async fn start(&self, facts: EngineFacts) -> Result<EngineSnapshot, EngineServiceError>;
    /// 停止入口进程并等待回收；入口记录保留以便再次启动
    async fn stop(&self, entry_id: &str) -> Result<EngineSnapshot, EngineServiceError>;
    /// 列出当前已知入口及其最新观测
    async fn list(&self) -> Result<Vec<EngineSnapshot>, EngineServiceError>;
}
