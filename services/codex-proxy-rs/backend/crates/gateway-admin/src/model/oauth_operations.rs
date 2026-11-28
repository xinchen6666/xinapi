//! 管理员授权操作的冻结归属、专用出口与只读回执

use super::{
    MutationActor,
    managed_routes::{RouteNode, RouteObservation},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartOAuthOperation {
    pub operation_id: String,
    pub provider: String,
    pub name: String,
    pub account_id: Option<String>,
    pub source_id: Option<String>,
    pub node_id: Option<String>,
}

/// Token 只随本次请求传递，持久操作保留归属与节点事实
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportManagedToken {
    pub operation_id: String,
    pub provider: String,
    pub name: String,
    pub source_id: String,
    pub node_id: String,
    pub access_token: String,
}

/// 私有操作事实仅经服务层投影，flow与入口凭据保持服务端持有
#[derive(Clone)]
pub struct OAuthOperation {
    pub request: StartOAuthOperation,
    pub owner: MutationActor,
    pub planned_account_id: String,
    pub node: RouteNode,
    pub observation: Option<RouteObservation>,
    pub source_revision: u64,
    pub proxy_id: String,
    pub proxy_revision: Option<u64>,
    pub route_generation: u64,
    pub endpoint: Option<String>,
    pub phase: String,
    pub flow_id: Option<String>,
    pub authorization_url: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub reason: Option<String>,
    pub receipt_confirmed: bool,
}
impl OAuthOperation {
    #[must_use]
    pub fn is_new(&self) -> bool {
        self.request.account_id.is_none()
    }
    #[must_use]
    pub fn view(&self, with_authorization_url: bool) -> OAuthOperationView {
        let status = if self.phase == "completed" {
            if self.receipt_confirmed {
                "completed"
            } else {
                "unknown"
            }
        } else if matches!(self.phase.as_str(), "cancelled" | "failed") {
            self.phase.as_str()
        } else if self.expires_at <= Utc::now() {
            if matches!(self.phase.as_str(), "preparing" | "pending") {
                "expired"
            } else {
                "unknown"
            }
        } else {
            match self.phase.as_str() {
                "pending" => "pending",
                "failed" => "failed",
                "cancelled" => "cancelled",
                _ => "unknown",
            }
        };
        OAuthOperationView {
            operation_id: self.request.operation_id.clone(),
            status: status.into(),
            provider: Some(self.request.provider.clone()),
            account_id: Some(self.planned_account_id.clone()),
            expires_at: Some(self.expires_at),
            source_id: Some(self.node.source_id.clone()),
            node_id: Some(self.node.node_id.clone()),
            version_id: Some(self.node.version_id.clone()),
            proxy_id: Some(self.proxy_id.clone()),
            route_generation: self.route_generation,
            authorization_url: if with_authorization_url && status == "pending" {
                self.authorization_url.clone()
            } else {
                None
            },
            reason: self.reason.clone(),
        }
    }
}
impl std::fmt::Debug for OAuthOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthOperation")
            .field("operation_id", &self.request.operation_id)
            .field("phase", &self.phase)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Serialize)]
pub struct OAuthOperationView {
    pub operation_id: String,
    pub status: String,
    pub provider: Option<String>,
    pub account_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub source_id: Option<String>,
    pub node_id: Option<String>,
    pub version_id: Option<String>,
    pub proxy_id: Option<String>,
    pub route_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
impl std::fmt::Debug for OAuthOperationView {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OAuthOperationView")
            .field("operation_id", &self.operation_id)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
impl OAuthOperationView {
    #[must_use]
    pub fn unknown(operation_id: String) -> Self {
        Self {
            operation_id,
            status: "unknown".into(),
            provider: None,
            account_id: None,
            expires_at: None,
            source_id: None,
            node_id: None,
            version_id: None,
            proxy_id: None,
            route_generation: 0,
            authorization_url: None,
            reason: None,
        }
    }
}
