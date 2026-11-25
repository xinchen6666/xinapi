//! 账号受管路由、候选出口观测与可读回操作回执

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteNode {
    pub source_id: String,
    pub node_id: String,
    pub version_id: String,
    pub node_name: String,
    pub protocol: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteObservation {
    pub success: bool,
    pub latency_ms: u64,
    pub exit_ip: Option<std::net::IpAddr>,
    pub exit_ipv4: Option<std::net::Ipv4Addr>,
    pub exit_ipv6: Option<std::net::Ipv6Addr>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePhase {
    Preparing,
    Ready,
    Applying,
    Applied,
    Failed,
}
impl RoutePhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Ready => "ready",
            Self::Applying => "applying",
            Self::Applied => "applied",
            Self::Failed => "failed",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOperation {
    pub operation_id: String,
    pub account_id: String,
    pub route_id: String,
    pub expected_generation: u64,
    pub expected_proxy_id: Option<String>,
    pub candidate: RouteNode,
    pub phase: RoutePhase,
    pub observation: Option<RouteObservation>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ManagedRoute {
    pub route_id: String,
    pub account_id: String,
    pub proxy_id: Option<String>,
    pub account_proxy_id: Option<String>,
    pub generation: u64,
    pub current: Option<RouteNode>,
    pub observation: Option<RouteObservation>,
    pub applying_operation_id: Option<String>,
    pub endpoint: Option<String>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone)]
pub struct BeginRoute {
    pub operation_id: String,
    pub account_id: String,
    pub expected_generation: u64,
    pub candidate: RouteNode,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareRoute {
    pub operation_id: String,
    pub account_id: String,
    pub expected_generation: u64,
    pub source_id: String,
    pub node_id: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct RouteReceipt {
    pub operation: RouteOperation,
    pub route: Option<ManagedRoute>,
    pub engine: Option<super::engines::EngineView>,
    pub requires_reconciliation: bool,
}
