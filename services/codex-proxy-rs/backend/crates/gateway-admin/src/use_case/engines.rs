//! 引擎入口用例：把已批准节点交给 Host 监督，并只发布公开观测

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    model::engines::{EngineFacts, EngineServiceError, EngineSnapshot, EngineView},
    ports::{
        engine::EngineHost, store::AdminStoreErrorKind, subscription_sources::SubscriptionStore,
    },
};

#[async_trait]
pub trait EnginesService: Send + Sync {
    async fn start(&self, source_id: &str, node_id: &str)
    -> Result<EngineView, EngineServiceError>;
    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError>;
    async fn list(&self) -> Result<Vec<EngineView>, EngineServiceError>;
}

pub(crate) struct DefaultEnginesService {
    store: Option<Arc<dyn SubscriptionStore>>,
    host: Arc<dyn EngineHost>,
}

impl DefaultEnginesService {
    pub(crate) fn new(
        store: Option<Arc<dyn SubscriptionStore>>,
        host: Arc<dyn EngineHost>,
    ) -> Self {
        Self { store, host }
    }

    fn store(&self) -> Result<&Arc<dyn SubscriptionStore>, EngineServiceError> {
        self.store.as_ref().ok_or(EngineServiceError::Unavailable)
    }
}

/// 同一来源节点在重启后复用同一入口
#[must_use]
pub(crate) fn entry_id(source_id: &str, node_id: &str) -> String {
    format!("{source_id}:{node_id}")
}

#[async_trait]
impl EnginesService for DefaultEnginesService {
    async fn start(
        &self,
        source_id: &str,
        node_id: &str,
    ) -> Result<EngineView, EngineServiceError> {
        let document = self
            .store()?
            .load(source_id)
            .await
            .map_err(|error| match error.kind() {
                AdminStoreErrorKind::NotFound => EngineServiceError::SourceMissing,
                _ => EngineServiceError::Store,
            })?;
        let node = document
            .nodes
            .iter()
            .find(|item| item.id == node_id)
            .ok_or(EngineServiceError::NodeMissing)?;
        if !node.available {
            return Err(EngineServiceError::NodeInactive);
        }
        if !node.supported {
            return Err(EngineServiceError::NodeUnsupported);
        }
        let version = document
            .versions
            .iter()
            .find(|item| item.id == node.version_id)
            .ok_or(EngineServiceError::Store)?;
        let facts = EngineFacts {
            entry_id: entry_id(source_id, node_id),
            source_id: source_id.to_owned(),
            node_id: node_id.to_owned(),
            node_name: node.name.clone(),
            protocol: node.protocol.clone(),
            connection: version.connection.0.clone(),
        };
        let snapshot: EngineSnapshot = self.host.start(facts).await?;
        Ok(snapshot.into())
    }

    async fn stop(&self, entry_id: &str) -> Result<EngineView, EngineServiceError> {
        let snapshot = self.host.stop(entry_id).await?;
        Ok(snapshot.into())
    }

    async fn list(&self) -> Result<Vec<EngineView>, EngineServiceError> {
        let snapshots = self.host.list().await?;
        Ok(snapshots.into_iter().map(EngineView::from).collect())
    }
}
