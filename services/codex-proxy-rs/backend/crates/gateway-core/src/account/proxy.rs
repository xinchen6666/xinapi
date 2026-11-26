//! 账号出口连接、托管发送要求与纯准入合同

use crate::upstream::ProxyEndpoint;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};
use url::Url;

/// 必须出口的事实独立于当前连接可用性，故障时保留拒绝状态
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManagedEgressRequirement {
    Account {
        account_id: String,
        proxy_id: String,
        generation: u64,
    },
    Authorization {
        operation_id: String,
        account_id: String,
    },
    Reservation {
        entry_id: String,
        source_id: String,
        node_id: String,
        version_id: String,
    },
    Blocked {
        account_id: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Managed egress is unavailable.")]
pub struct ManagedEgressDenied;

/// Store复核绑定后返回固定节点身份，Host再校验实际进程
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedEgressTarget {
    pub entry_id: String,
    pub source_id: String,
    pub node_id: String,
    pub version_id: String,
}
pub trait ManagedEgressState: Send + Sync {
    fn authorize_account<'a>(
        &'a self,
        account_id: &'a str,
        proxy: Option<&'a OutboundProxy>,
    ) -> BoxFuture<'a, Result<Option<ManagedEgressTarget>, ManagedEgressDenied>>;
    fn authorize<'a>(
        &'a self,
        proxy: &'a OutboundProxy,
    ) -> BoxFuture<'a, Result<ManagedEgressTarget, ManagedEgressDenied>>;
}
pub trait ManagedEgressGate: Send + Sync {
    fn admit_account<'a>(
        &'a self,
        account_id: &'a str,
        proxy: Option<&'a OutboundProxy>,
    ) -> BoxFuture<'a, Result<(), ManagedEgressDenied>>;
    fn admit<'a>(
        &'a self,
        proxy: &'a OutboundProxy,
    ) -> BoxFuture<'a, Result<(), ManagedEgressDenied>>;
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OutboundProxy {
    url: Option<Url>,
    max_websocket_connections: u32,
    route_generation: u64,
    managed: Option<Arc<ManagedEgressRequirement>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid outbound proxy; expected http, https, socks5 or socks5h URL with a host and port")]
pub struct InvalidOutboundProxy;
impl OutboundProxy {
    pub fn parse(value: &str) -> Result<Self, InvalidOutboundProxy> {
        if value.len() > 4096 || value.chars().any(char::is_control) {
            return Err(InvalidOutboundProxy);
        }
        let url = Url::parse(value).map_err(|_| InvalidOutboundProxy)?;
        if !matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h")
            || url.host_str().is_none()
            || url.port_or_known_default().is_none_or(|p| p == 0)
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(InvalidOutboundProxy);
        }
        Ok(Self {
            url: Some(url),
            max_websocket_connections: 0,
            route_generation: 0,
            managed: None,
        })
    }
    #[must_use]
    pub fn blocked_managed(account_id: String) -> Self {
        Self {
            url: None,
            max_websocket_connections: 0,
            route_generation: 0,
            managed: Some(Arc::new(ManagedEgressRequirement::Blocked { account_id })),
        }
    }
    #[must_use]
    pub fn with_managed_requirement(mut self, requirement: ManagedEgressRequirement) -> Self {
        self.managed = Some(Arc::new(requirement));
        self
    }
    #[must_use]
    pub fn managed_requirement(&self) -> Option<&ManagedEgressRequirement> {
        self.managed.as_deref()
    }
    #[must_use]
    pub const fn is_managed(&self) -> bool {
        self.managed.is_some() || self.route_generation > 0
    }
    pub fn ensure_sendable(&self) -> Result<(), ManagedEgressDenied> {
        if self.url.is_none()
            || matches!(
                self.managed.as_deref(),
                Some(ManagedEgressRequirement::Blocked { .. })
            )
        {
            Err(ManagedEgressDenied)
        } else {
            Ok(())
        }
    }
    #[must_use]
    pub fn with_max_websocket_connections(mut self, limit: u32) -> Self {
        self.max_websocket_connections = limit;
        self
    }
    #[must_use]
    pub const fn max_websocket_connections(&self) -> u32 {
        self.max_websocket_connections
    }
    #[must_use]
    pub fn with_route_generation(mut self, generation: u64) -> Self {
        self.route_generation = generation;
        self
    }
    #[must_use]
    pub const fn route_generation(&self) -> u64 {
        self.route_generation
    }
    /// 拒绝状态返回空值，任何URL解析均明确失败
    #[must_use]
    pub fn expose_url(&self) -> &str {
        self.url.as_ref().map_or("", Url::as_str)
    }
    #[must_use]
    pub fn dial_endpoint(&self) -> ProxyEndpoint {
        match &self.url {
            Some(url) => ProxyEndpoint::new(
                url.host_str().expect("validated host"),
                url.port_or_known_default().expect("validated port"),
            ),
            None => ProxyEndpoint::new("managed-egress-denied.invalid", 0),
        }
    }
    #[must_use]
    pub fn matches_endpoint(&self, endpoint: &ProxyEndpoint) -> bool {
        self.url.as_ref().is_some_and(|url| {
            endpoint.matches(
                url.host_str().expect("validated host"),
                url.port_or_known_default().expect("validated port"),
            )
        })
    }
    #[must_use]
    pub fn endpoint(&self) -> String {
        let Some(mut url) = self.url.clone() else {
            return String::new();
        };
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.to_string()
    }
}
impl fmt::Debug for OutboundProxy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OutboundProxy(<redacted>)")
    }
}
