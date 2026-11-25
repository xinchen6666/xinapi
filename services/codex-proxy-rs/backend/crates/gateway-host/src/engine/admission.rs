//! 组合数据库当前许可与实际引擎事实的托管发送准入

use super::EngineSupervisor;
use futures::future::BoxFuture;
use gateway_core::account::{
    ManagedEgressDenied, ManagedEgressGate, ManagedEgressRequirement, ManagedEgressState,
    OutboundProxy,
};
use std::sync::Arc;

pub(crate) struct StrictEgressGate {
    pub(crate) state: Arc<dyn ManagedEgressState>,
    pub(crate) engines: Arc<EngineSupervisor>,
}
impl ManagedEgressGate for StrictEgressGate {
    fn admit_account<'a>(
        &'a self,
        account_id: &'a str,
        proxy: Option<&'a OutboundProxy>,
    ) -> BoxFuture<'a, Result<(), ManagedEgressDenied>> {
        Box::pin(async move {
            let target = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                self.state.authorize_account(account_id, proxy),
            )
            .await
            .map_err(|_| ManagedEgressDenied)??;
            if let Some(target) = target {
                self.engines
                    .authorize_egress(&target, proxy.ok_or(ManagedEgressDenied)?)
                    .await
            } else if proxy.is_some_and(OutboundProxy::is_managed) {
                Err(ManagedEgressDenied)
            } else {
                Ok(())
            }
        })
    }
    fn admit<'a>(
        &'a self,
        proxy: &'a OutboundProxy,
    ) -> BoxFuture<'a, Result<(), ManagedEgressDenied>> {
        Box::pin(async move {
            proxy.ensure_sendable()?;
            let target = match proxy.managed_requirement() {
                Some(ManagedEgressRequirement::Blocked { .. }) => return Err(ManagedEgressDenied),
                Some(ManagedEgressRequirement::Reservation { .. })
                | Some(ManagedEgressRequirement::Account { .. }) => tokio::time::timeout(
                    std::time::Duration::from_secs(3),
                    self.state.authorize(proxy),
                )
                .await
                .map_err(|_| ManagedEgressDenied)??,
                None => return Err(ManagedEgressDenied),
            };
            self.engines.authorize_egress(&target, proxy).await
        })
    }
}
