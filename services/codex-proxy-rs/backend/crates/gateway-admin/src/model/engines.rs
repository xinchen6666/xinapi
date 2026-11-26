//! 受管引擎的归属、启动事实与公开观测

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 账号与授权预留项分别拥有独立入口
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineOwnerKind {
    Account,
    Reservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineOwner {
    pub kind: EngineOwnerKind,
    pub id: String,
}

impl EngineOwner {
    pub fn normalize(mut self) -> Result<Self, EngineServiceError> {
        self.id = match self.kind {
            EngineOwnerKind::Account => gateway_core::account::ProviderAccountId::new(self.id)
                .map_err(|_| EngineServiceError::OwnerInvalid)?
                .as_str()
                .to_owned(),
            EngineOwnerKind::Reservation => uuid::Uuid::parse_str(&self.id)
                .map_err(|_| EngineServiceError::OwnerInvalid)?
                .to_string(),
        };
        Ok(self)
    }

    /// 账号标识用URL安全编码，入口目录保持单一可逆命名
    #[must_use]
    pub fn entry_id(&self) -> String {
        use base64::Engine as _;
        match self.kind {
            EngineOwnerKind::Account => format!(
                "account-{}",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.id.as_bytes())
            ),
            EngineOwnerKind::Reservation => format!("reservation-{}", self.id),
        }
    }
}

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

/// 私有连接仅交给 Host 生成配置，身份与批准版本来自 Admin
#[derive(Clone)]
pub struct EngineFacts {
    pub entry_id: String,
    pub owner: EngineOwner,
    pub source_id: String,
    pub node_id: String,
    pub version_id: String,
    pub node_name: String,
    pub protocol: String,
    pub connection: serde_json::Value,
}
impl std::fmt::Debug for EngineFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineFacts")
            .field("entry_id", &self.entry_id)
            .field("owner", &self.owner)
            .field("source_id", &self.source_id)
            .field("node_id", &self.node_id)
            .field("version_id", &self.version_id)
            .field("connection", &"[REDACTED]")
            .finish()
    }
}

/// 公开观测只带身份与资源值，入口认证材料留在 Host
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub entry_id: String,
    pub owner: Option<EngineOwner>,
    pub source_id: String,
    pub node_id: String,
    pub version_id: String,
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
    pub occupies_slot: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct EngineCapacity {
    pub limit: usize,
    pub used: usize,
    pub available: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineList {
    pub items: Vec<EngineView>,
    pub capacity: EngineCapacity,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineView {
    pub entry_id: String,
    pub owner: Option<EngineOwner>,
    pub source_id: String,
    pub node_id: String,
    pub version_id: String,
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
    pub occupies_slot: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub reclaimable: bool,
    pub stoppable: bool,
}
impl From<EngineSnapshot> for EngineView {
    fn from(s: EngineSnapshot) -> Self {
        Self {
            entry_id: s.entry_id,
            owner: s.owner,
            source_id: s.source_id,
            node_id: s.node_id,
            version_id: s.version_id,
            node_name: s.node_name,
            protocol: s.protocol,
            listen_host: s.listen_host,
            listen_port: s.listen_port,
            state: s.state.as_str(),
            pid: s.pid,
            started_at: s.started_at,
            rss_bytes: s.rss_bytes,
            cpu_seconds: s.cpu_seconds,
            cold_start_ms: s.cold_start_ms,
            last_error: s.last_error,
            occupies_slot: s.occupies_slot,
            expires_at: None,
            reclaimable: false,
            stoppable: false,
        }
    }
}

/// 固定失败分类用于管理端提示和程序分支
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EngineServiceError {
    #[error("engine binary is missing")]
    BinaryMissing,
    #[error("engine artifact identity verification failed")]
    ArtifactIdentity,
    #[error("engine capacity is reached")]
    Capacity,
    #[error("engine entry port is occupied")]
    PortConflict,
    #[error("engine start timed out")]
    StartTimeout,
    #[error("engine operation was cancelled")]
    Cancelled,
    #[error("engine host is shutting down")]
    ShuttingDown,
    #[error("engine binding differs from its reserved entry")]
    BindingConflict,
    #[error("engine entry is unknown")]
    UnknownEntry,
    #[error("engine owner is invalid or absent")]
    OwnerInvalid,
    #[error("engine registry is unavailable")]
    Registry,
    #[error("engine host is unavailable")]
    Unavailable,
    #[error("engine source entry is missing")]
    SourceMissing,
    #[error("engine node entry is missing")]
    NodeMissing,
    #[error("engine node is inactive")]
    NodeInactive,
    #[error("engine node options are unsupported")]
    NodeUnsupported,
    #[error("approved engine node version changed")]
    NodeVersionChanged,
    #[error("engine node connection test timed out")]
    NodeTestTimeout,
    #[error("only a published account engine can be stopped")]
    StopForbidden,
    #[error("engine process failed")]
    ProcessFailed,
    #[error("engine storage is unavailable")]
    Store,
    #[error("engine configuration is invalid")]
    Configuration,
}
impl EngineServiceError {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::BinaryMissing => "engine_binary_missing",
            Self::ArtifactIdentity => "engine_artifact_identity",
            Self::Capacity => "engine_capacity_reached",
            Self::PortConflict => "engine_port_conflict",
            Self::StartTimeout => "engine_start_timeout",
            Self::Cancelled => "engine_cancelled",
            Self::ShuttingDown => "engine_shutting_down",
            Self::BindingConflict => "engine_binding_conflict",
            Self::UnknownEntry => "engine_entry_unknown",
            Self::OwnerInvalid => "engine_owner_invalid",
            Self::Registry => "engine_registry_unavailable",
            Self::Unavailable => "engine_host_unavailable",
            Self::SourceMissing => "engine_source_missing",
            Self::NodeMissing => "engine_node_missing",
            Self::NodeInactive => "engine_node_inactive",
            Self::NodeUnsupported => "engine_node_unsupported",
            Self::NodeVersionChanged => "engine_node_version_changed",
            Self::NodeTestTimeout => "engine_node_test_timeout",
            Self::StopForbidden => "engine_stop_forbidden",
            Self::ProcessFailed => "engine_process_failed",
            Self::Store => "engine_store_unavailable",
            Self::Configuration => "engine_configuration_invalid",
        }
    }
}
