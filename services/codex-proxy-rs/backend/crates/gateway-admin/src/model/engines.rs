//! 受管引擎（Mihomo）的启动事实与公开状态
//!
//! 连接文档只进入 Host 的配置生成；公开视图只带入口地址、进程与资源观测

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 监督器报告的进程状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    Starting,
    Running,
    Stopped,
    Failed,
}

impl EngineState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }
}

/// 启动一个入口所需的私有事实
#[derive(Clone)]
pub struct EngineFacts {
    pub entry_id: String,
    pub source_id: String,
    pub node_id: String,
    pub node_name: String,
    pub protocol: String,
    pub connection: serde_json::Value,
}

impl std::fmt::Debug for EngineFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineFacts")
            .field("entry_id", &self.entry_id)
            .field("source_id", &self.source_id)
            .field("node_id", &self.node_id)
            .field("node_name", &self.node_name)
            .field("protocol", &self.protocol)
            .field("connection", &"[REDACTED]")
            .finish()
    }
}

/// 监督器对单个入口的观测结果
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub entry_id: String,
    pub source_id: String,
    pub node_id: String,
    pub node_name: String,
    pub protocol: String,
    pub listen_host: String,
    pub listen_port: u16,
    pub state: EngineState,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub rss_bytes: Option<u64>,
    pub cpu_seconds: Option<f64>,
    pub cold_start_ms: Option<u64>,
    pub last_error: Option<String>,
}

/// 对外发布的引擎视图
#[derive(Debug, Clone, Serialize)]
pub struct EngineView {
    pub entry_id: String,
    pub source_id: String,
    pub node_id: String,
    pub node_name: String,
    pub protocol: String,
    pub listen_host: String,
    pub listen_port: u16,
    pub state: &'static str,
    pub pid: Option<u32>,
    pub started_at: Option<DateTime<Utc>>,
    pub rss_bytes: Option<u64>,
    pub cpu_seconds: Option<f64>,
    pub cold_start_ms: Option<u64>,
    pub last_error: Option<String>,
}

impl From<EngineSnapshot> for EngineView {
    fn from(snapshot: EngineSnapshot) -> Self {
        Self {
            entry_id: snapshot.entry_id,
            source_id: snapshot.source_id,
            node_id: snapshot.node_id,
            node_name: snapshot.node_name,
            protocol: snapshot.protocol,
            listen_host: snapshot.listen_host,
            listen_port: snapshot.listen_port,
            state: snapshot.state.as_str(),
            pid: snapshot.pid,
            started_at: snapshot.started_at,
            rss_bytes: snapshot.rss_bytes,
            cpu_seconds: snapshot.cpu_seconds,
            cold_start_ms: snapshot.cold_start_ms,
            last_error: snapshot.last_error,
        }
    }
}

/// 引擎用例的稳定失败分类；对外映射固定 reason 与 HTTP 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineServiceError {
    #[error("engine binary is missing")]
    BinaryMissing,
    #[error("engine capacity is reached")]
    Capacity,
    #[error("engine entry port is occupied")]
    PortConflict,
    #[error("engine start timed out")]
    StartTimeout,
    #[error("engine entry is unknown")]
    UnknownEntry,
    #[error("engine host is unavailable")]
    Unavailable,
    #[error("engine source entry is missing")]
    SourceMissing,
    #[error("engine node entry is missing")]
    NodeMissing,
    #[error("engine node is inactive")]
    NodeInactive,
    #[error("engine node protocol is unsupported")]
    NodeUnsupported,
    #[error("engine process failed")]
    ProcessFailed,
    #[error("engine storage is unavailable")]
    Store,
}

impl EngineServiceError {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::BinaryMissing => "engine_binary_missing",
            Self::Capacity => "engine_capacity_reached",
            Self::PortConflict => "engine_port_conflict",
            Self::StartTimeout => "engine_start_timeout",
            Self::UnknownEntry => "engine_entry_unknown",
            Self::Unavailable => "engine_host_unavailable",
            Self::SourceMissing => "engine_source_missing",
            Self::NodeMissing => "engine_node_missing",
            Self::NodeInactive => "engine_node_inactive",
            Self::NodeUnsupported => "engine_node_unsupported",
            Self::ProcessFailed => "engine_process_failed",
            Self::Store => "engine_store_unavailable",
        }
    }
}
