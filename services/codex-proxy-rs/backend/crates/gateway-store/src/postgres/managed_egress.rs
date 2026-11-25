//! 托管发送前的实时绑定、出口代次与来源可用性核对

use std::sync::Arc;

use futures::future::BoxFuture;
use gateway_admin::{
    model::{
        engines::{EngineOwner, EngineOwnerKind},
        managed_routes::RouteNode,
    },
    ports::subscription_sources::SubscriptionStore,
};
use gateway_core::account::{
    ManagedEgressDenied, ManagedEgressRequirement, ManagedEgressState, ManagedEgressTarget,
    OutboundProxy,
};
use sqlx::{PgPool, types::Json};

#[derive(Clone)]
pub struct PgManagedEgressState {
    pool: PgPool,
    sources: Option<Arc<dyn SubscriptionStore>>,
}

#[derive(sqlx::FromRow)]
struct AccountFacts {
    required: bool,
    enabled: bool,
    account_proxy_id: Option<String>,
    account_proxy_url: Option<String>,
    proxy_id: Option<String>,
    proxy_enabled: Option<bool>,
    proxy_url: Option<String>,
    proxy_generation: Option<i64>,
    route_proxy_id: Option<String>,
    route_generation: Option<i64>,
    applying: bool,
    current_node: Option<Json<RouteNode>>,
}

impl PgManagedEgressState {
    #[must_use]
    pub fn new(pool: PgPool, sources: Option<Arc<dyn SubscriptionStore>>) -> Self {
        Self { pool, sources }
    }

    async fn account_facts(&self, account_id: &str) -> Result<AccountFacts, ManagedEgressDenied> {
        sqlx::query_as::<_, AccountFacts>(
            "select a.managed_egress_required as required, a.enabled,
                    a.outbound_proxy_id as account_proxy_id,
                    a.outbound_proxy_url as account_proxy_url,
                    p.id as proxy_id, p.enabled as proxy_enabled, p.proxy_url,
                    p.route_generation as proxy_generation,
                    r.proxy_id as route_proxy_id, r.generation as route_generation,
                    (r.applying_operation_id is not null) as applying, r.current_node
               from provider_accounts a
               left join outbound_proxies p on p.id=a.outbound_proxy_id
               left join managed_routes r on r.account_id=a.id
              where a.id=$1",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ManagedEgressDenied)?
        .ok_or(ManagedEgressDenied)
    }

    async fn source_available(
        &self,
        target: &ManagedEgressTarget,
    ) -> Result<(), ManagedEgressDenied> {
        let document = self
            .sources
            .as_ref()
            .ok_or(ManagedEgressDenied)?
            .load(&target.source_id)
            .await
            .map_err(|_| ManagedEgressDenied)?;
        if document.summary.id != target.source_id
            || !document
                .nodes
                .iter()
                .any(|node| node.id == target.node_id && node.available)
            || !document
                .versions
                .iter()
                .any(|version| version.id == target.version_id && version.node_id == target.node_id)
        {
            return Err(ManagedEgressDenied);
        }
        Ok(())
    }

    async fn authorize_bound(
        &self,
        account_id: &str,
        facts: AccountFacts,
        proxy: &OutboundProxy,
    ) -> Result<ManagedEgressTarget, ManagedEgressDenied> {
        proxy.ensure_sendable()?;
        let Some(ManagedEgressRequirement::Account {
            account_id: owner,
            proxy_id,
            generation,
        }) = proxy.managed_requirement()
        else {
            return Err(ManagedEgressDenied);
        };
        if owner != account_id
            || !facts.required
            || !facts.enabled
            || facts.applying
            || facts.proxy_enabled != Some(true)
            || facts.account_proxy_id.as_deref() != Some(proxy_id)
            || facts.proxy_id.as_deref() != Some(proxy_id)
            || facts.route_proxy_id.as_deref() != Some(proxy_id)
            || facts.proxy_url.as_deref() != Some(proxy.expose_url())
            || facts.account_proxy_url.as_deref() != Some(proxy.expose_url())
            || *generation == 0
            || proxy.route_generation() != *generation
            || facts
                .proxy_generation
                .and_then(|value| u64::try_from(value).ok())
                != Some(*generation)
            || facts
                .route_generation
                .and_then(|value| u64::try_from(value).ok())
                != Some(*generation)
        {
            return Err(ManagedEgressDenied);
        }
        let node = facts.current_node.ok_or(ManagedEgressDenied)?.0;
        let owner = EngineOwner {
            kind: EngineOwnerKind::Account,
            id: account_id.to_owned(),
        }
        .normalize()
        .map_err(|_| ManagedEgressDenied)?;
        let target = ManagedEgressTarget {
            entry_id: owner.entry_id(),
            source_id: node.source_id,
            node_id: node.node_id,
            version_id: node.version_id,
        };
        self.source_available(&target).await?;
        Ok(target)
    }
}

impl ManagedEgressState for PgManagedEgressState {
    fn authorize_account<'a>(
        &'a self,
        account_id: &'a str,
        proxy: Option<&'a OutboundProxy>,
    ) -> BoxFuture<'a, Result<Option<ManagedEgressTarget>, ManagedEgressDenied>> {
        Box::pin(async move {
            let facts = self.account_facts(account_id).await?;
            if !facts.required {
                return Ok(None);
            }
            let target = self
                .authorize_bound(account_id, facts, proxy.ok_or(ManagedEgressDenied)?)
                .await?;
            Ok(Some(target))
        })
    }

    fn authorize<'a>(
        &'a self,
        proxy: &'a OutboundProxy,
    ) -> BoxFuture<'a, Result<ManagedEgressTarget, ManagedEgressDenied>> {
        Box::pin(async move {
            proxy.ensure_sendable()?;
            match proxy.managed_requirement() {
                Some(ManagedEgressRequirement::Account { account_id, .. }) => {
                    let facts = self.account_facts(account_id).await?;
                    self.authorize_bound(account_id, facts, proxy).await
                }
                Some(ManagedEgressRequirement::Reservation {
                    entry_id,
                    source_id,
                    node_id,
                    version_id,
                }) => {
                    let id = entry_id
                        .strip_prefix("reservation-")
                        .ok_or(ManagedEgressDenied)?;
                    let owner = EngineOwner {
                        kind: EngineOwnerKind::Reservation,
                        id: id.to_owned(),
                    }
                    .normalize()
                    .map_err(|_| ManagedEgressDenied)?;
                    if owner.entry_id() != *entry_id {
                        return Err(ManagedEgressDenied);
                    }
                    let target = ManagedEgressTarget {
                        entry_id: entry_id.clone(),
                        source_id: source_id.clone(),
                        node_id: node_id.clone(),
                        version_id: version_id.clone(),
                    };
                    self.source_available(&target).await?;
                    Ok(target)
                }
                _ => Err(ManagedEgressDenied),
            }
        })
    }
}
