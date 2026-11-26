//! 按账号或授权预留项解析已批准节点，并把规范身份交给 Host 监督

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    model::engines::{EngineFacts, EngineOwner, EngineOwnerKind, EngineServiceError, EngineView},
    ports::{
        engine::EngineHost,
        store::{AccountStore, AdminStoreErrorKind},
        subscription_sources::SubscriptionStore,
    },
};

#[async_trait]
pub trait EnginesService: Send + Sync {
    async fn start(
        &self,
        owner: EngineOwner,
        source_id: &str,
        node_id: &str,
    ) -> Result<EngineView, EngineServiceError>;
    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError>;
    async fn list(&self) -> Result<Vec<EngineView>, EngineServiceError>;
}

pub(crate) struct DefaultEnginesService {
    store: Option<Arc<dyn SubscriptionStore>>,
    accounts: Arc<dyn AccountStore>,
    host: Arc<dyn EngineHost>,
}

impl DefaultEnginesService {
    pub(crate) fn new(
        store: Option<Arc<dyn SubscriptionStore>>,
        accounts: Arc<dyn AccountStore>,
        host: Arc<dyn EngineHost>,
    ) -> Self {
        Self {
            store,
            accounts,
            host,
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
        let version = document
            .versions
            .iter()
            .find(|item| {
                Uuid::parse_str(&item.id).ok() == Some(current_version)
                    && Uuid::parse_str(&item.node_id).ok() == Some(node_id)
            })
            .ok_or(EngineServiceError::Store)?;
        let version_id = Uuid::parse_str(&version.id).map_err(|_| EngineServiceError::Store)?;
        let facts = EngineFacts {
            entry_id: owner.entry_id(),
            owner,
            source_id: source.to_string(),
            node_id: node_id.to_string(),
            version_id: version_id.to_string(),
            node_name: node.name.clone(),
            protocol: node.protocol.clone(),
            connection: version.connection.0.clone(),
        };
        Ok(self.host.start(facts).await?.into())
    }

    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError> {
        Ok(self.host.stop(entry_id).await?.into())
    }

    async fn list(&self) -> Result<Vec<EngineView>, EngineServiceError> {
        let snapshots = self.host.list().await?;
        Ok(snapshots.into_iter().map(EngineView::from).collect())
    }
}
