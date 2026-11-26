//! 按账号或授权预留项解析已批准节点，并把规范身份交给 Host 监督

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    model::{
        AdminErrorKind, MutationContext,
        audit::{MutationAuditIntent, MutationAuditOperation},
        engines::{
            EngineFacts, EngineList, EngineOwner, EngineOwnerKind, EngineServiceError, EngineView,
        },
        managed_routes::{RouteNode, RouteObservation},
    },
    ports::{
        engine::EngineHost,
        oauth_operations::OAuthOperationStore,
        proxy::ProxyProbe,
        store::{AccountStore, AdminStoreErrorKind, AuthStore},
        subscription_sources::SubscriptionStore,
    },
};

use super::managed_routes::{DefaultManagedRoutesService, ManagedRoutesService};

#[async_trait]
pub trait EnginesService: Send + Sync {
    async fn start(
        &self,
        owner: EngineOwner,
        source_id: &str,
        node_id: &str,
    ) -> Result<EngineView, EngineServiceError>;
    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError>;
    async fn stop_account(
        &self,
        entry_id: &str,
        context: &MutationContext,
    ) -> Result<EngineView, EngineServiceError>;
    async fn list(&self) -> Result<EngineList, EngineServiceError>;
    async fn test_node(
        &self,
        source_id: &str,
        node_id: &str,
        version_id: &str,
        context: &MutationContext,
    ) -> Result<RouteObservation, EngineServiceError>;
}

pub(crate) struct DefaultEnginesService {
    store: Option<Arc<dyn SubscriptionStore>>,
    accounts: Arc<dyn AccountStore>,
    host: Arc<dyn EngineHost>,
    routes: Arc<DefaultManagedRoutesService>,
    oauth: Option<Arc<dyn OAuthOperationStore>>,
    probe: Arc<dyn ProxyProbe>,
    audit: Arc<dyn AuthStore>,
}

impl DefaultEnginesService {
    pub(crate) fn new(
        store: Option<Arc<dyn SubscriptionStore>>,
        accounts: Arc<dyn AccountStore>,
        host: Arc<dyn EngineHost>,
        routes: Arc<DefaultManagedRoutesService>,
        oauth: Option<Arc<dyn OAuthOperationStore>>,
        probe: Arc<dyn ProxyProbe>,
        audit: Arc<dyn AuthStore>,
    ) -> Self {
        Self {
            store,
            accounts,
            host,
            routes,
            oauth,
            probe,
            audit,
        }
    }

    fn store(&self) -> Result<&Arc<dyn SubscriptionStore>, EngineServiceError> {
        self.store.as_ref().ok_or(EngineServiceError::Unavailable)
    }

    async fn owner(&self, owner: EngineOwner) -> Result<EngineOwner, EngineServiceError> {
        let owner = owner.normalize()?;
        if owner.kind == EngineOwnerKind::Account
            && !self
                .accounts
                .account_exists(&owner.id)
                .await
                .map_err(|_| EngineServiceError::Store)?
        {
            return Err(EngineServiceError::OwnerInvalid);
        }
        Ok(owner)
    }

    async fn approved_facts(
        &self,
        owner: EngineOwner,
        source_id: &str,
        node_id: &str,
        expected_version: Option<Uuid>,
    ) -> Result<EngineFacts, EngineServiceError> {
        let requested_source =
            Uuid::parse_str(source_id).map_err(|_| EngineServiceError::SourceMissing)?;
        let requested_node =
            Uuid::parse_str(node_id).map_err(|_| EngineServiceError::NodeMissing)?;
        let document = self
            .store()?
            .load(&requested_source.to_string())
            .await
            .map_err(|error| match error.kind() {
                AdminStoreErrorKind::NotFound => EngineServiceError::SourceMissing,
                _ => EngineServiceError::Store,
            })?;
        let source =
            Uuid::parse_str(&document.summary.id).map_err(|_| EngineServiceError::Store)?;
        if source != requested_source {
            return Err(EngineServiceError::Store);
        }
        let node = document
            .nodes
            .iter()
            .find(|item| Uuid::parse_str(&item.id).ok() == Some(requested_node))
            .ok_or(EngineServiceError::NodeMissing)?;
        if !node.available {
            return Err(EngineServiceError::NodeInactive);
        }
        if !node.supported {
            return Err(EngineServiceError::NodeUnsupported);
        }
        let node_id = Uuid::parse_str(&node.id).map_err(|_| EngineServiceError::Store)?;
        let current_version =
            Uuid::parse_str(&node.version_id).map_err(|_| EngineServiceError::Store)?;
        if expected_version.is_some_and(|version| version != current_version) {
            return Err(EngineServiceError::NodeVersionChanged);
        }
        let version = document
            .versions
            .iter()
            .find(|item| {
                Uuid::parse_str(&item.id).ok() == Some(current_version)
                    && Uuid::parse_str(&item.node_id).ok() == Some(node_id)
            })
            .ok_or(EngineServiceError::Store)?;
        let version_id = Uuid::parse_str(&version.id).map_err(|_| EngineServiceError::Store)?;
        Ok(EngineFacts {
            entry_id: owner.entry_id(),
            owner,
            source_id: source.to_string(),
            node_id: node_id.to_string(),
            version_id: version_id.to_string(),
            node_name: node.name.clone(),
            protocol: node.protocol.clone(),
            connection: version.connection.0.clone(),
        })
    }

    async fn published_node(
        &self,
        account_id: &str,
    ) -> Result<Option<RouteNode>, EngineServiceError> {
        let route = match self.routes.route(account_id).await {
            Ok(route) => route,
            Err(error) if error.kind() == AdminErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(EngineServiceError::Store),
        };
        let matches = route.generation > 0
            && route.proxy_id.is_some()
            && route.account_proxy_id == route.proxy_id
            && route.applying_operation_id.is_none();
        Ok(matches.then_some(route.current).flatten())
    }

    async fn audit(
        &self,
        operation: MutationAuditOperation,
        entity_ref: &str,
        context: &MutationContext,
    ) -> Result<(), EngineServiceError> {
        let event = MutationAuditIntent {
            operation,
            entity_ref,
        }
        .event(context, Vec::new());
        tokio::time::timeout(Duration::from_secs(3), self.audit.append_audit_event(event))
            .await
            .map_err(|_| EngineServiceError::Store)?
            .map_err(|_| EngineServiceError::Store)
    }
}

#[async_trait]
impl EnginesService for DefaultEnginesService {
    async fn start(
        &self,
        owner: EngineOwner,
        source_id: &str,
        node_id: &str,
    ) -> Result<EngineView, EngineServiceError> {
        let owner = self.owner(owner).await?;
        let facts = self.approved_facts(owner, source_id, node_id, None).await?;
        Ok(self.host.start(facts).await?.into())
    }

    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError> {
        Ok(self.host.stop(entry_id).await?.into())
    }

    async fn stop_account(
        &self,
        entry_id: &str,
        context: &MutationContext,
    ) -> Result<EngineView, EngineServiceError> {
        let engine = self
            .host
            .list()
            .await?
            .into_iter()
            .find(|engine| engine.entry_id == entry_id)
            .ok_or(EngineServiceError::UnknownEntry)?;
        let owner = engine
            .owner
            .as_ref()
            .filter(|owner| owner.kind == EngineOwnerKind::Account)
            .ok_or(EngineServiceError::StopForbidden)?;
        let _active = self
            .routes
            .lock_account(&owner.id)
            .map_err(|_| EngineServiceError::BindingConflict)?;
        let node = self
            .published_node(&owner.id)
            .await?
            .ok_or(EngineServiceError::StopForbidden)?;
        self.audit(MutationAuditOperation::EngineStop, &owner.id, context)
            .await?;
        Ok(self.host.stop_account(entry_id, &node).await?.into())
    }

    async fn list(&self) -> Result<EngineList, EngineServiceError> {
        let (snapshots, capacity) = self.host.inventory().await?;
        let account_ids: Vec<_> = snapshots
            .iter()
            .filter_map(|engine| engine.owner.as_ref())
            .filter(|owner| owner.kind == EngineOwnerKind::Account)
            .map(|owner| owner.id.clone())
            .collect();
        let mut reservations: BTreeMap<_, _> = if let Some(store) = &self.oauth {
            store
                .engine_reservations(&account_ids)
                .await
                .map_err(|_| EngineServiceError::Store)?
                .into_iter()
                .map(|reservation| (reservation.account_id.clone(), reservation))
                .collect()
        } else {
            BTreeMap::new()
        };
        let mut items = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            let mut view = EngineView::from(snapshot);
            if let Some(owner) = view
                .owner
                .as_ref()
                .filter(|owner| owner.kind == EngineOwnerKind::Account)
            {
                if let Some(reservation) = reservations.remove(&owner.id) {
                    view.owner = Some(EngineOwner {
                        kind: EngineOwnerKind::Reservation,
                        id: reservation.operation_id,
                    });
                    view.expires_at = Some(reservation.expires_at);
                    view.reclaimable = reservation.reclaimable;
                } else if view.occupies_slot {
                    view.stoppable = self.published_node(&owner.id).await?.is_some_and(|node| {
                        node.source_id == view.source_id
                            && node.node_id == view.node_id
                            && node.version_id == view.version_id
                    });
                }
            }
            items.push(view);
        }
        Ok(EngineList { items, capacity })
    }

    async fn test_node(
        &self,
        source_id: &str,
        node_id: &str,
        version_id: &str,
        context: &MutationContext,
    ) -> Result<RouteObservation, EngineServiceError> {
        let version =
            Uuid::parse_str(version_id).map_err(|_| EngineServiceError::NodeVersionChanged)?;
        let owner = EngineOwner {
            kind: EngineOwnerKind::Reservation,
            id: Uuid::new_v4().to_string(),
        };
        let facts = tokio::time::timeout(
            Duration::from_secs(3),
            self.approved_facts(owner.clone(), source_id, node_id, Some(version)),
        )
        .await
        .map_err(|_| EngineServiceError::Store)??;
        self.audit(
            MutationAuditOperation::EngineNodeTest,
            &facts.node_id,
            context,
        )
        .await?;
        let validate = Box::pin(async {
            self.approved_facts(owner, source_id, node_id, Some(version))
                .await
                .map(|_| ())
        });
        self.host
            .test_node(facts, self.probe.as_ref(), validate)
            .await
    }
}
