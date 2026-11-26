//! 受管账号路由的候选验证、明确发布与中断结果对账

use super::publish_committed;
use crate::{
    model::{
        AdminError, AdminErrorKind, MutationContext,
        engines::{EngineFacts, EngineOwner, EngineOwnerKind, EngineServiceError, EngineState},
        managed_routes::*,
    },
    ports::{
        engine::EngineHost,
        managed_routes::ManagedRouteStore,
        proxy::{ProbeRoute, ProxyProbe},
        store::AdminStoreError,
        subscription_sources::SubscriptionStore,
    },
};
use async_trait::async_trait;
use gateway_core::{account::ProviderAccountId, runtime::SnapshotControl};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

#[async_trait]
pub trait ManagedRoutesService: Send + Sync {
    async fn prepare(
        &self,
        command: PrepareRoute,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError>;
    async fn approve(
        &self,
        operation_id: &str,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError>;
    async fn reconcile(
        &self,
        operation_id: &str,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError>;
    async fn abort(
        &self,
        operation_id: &str,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError>;
    async fn operation(&self, operation_id: &str) -> Result<RouteReceipt, AdminError>;
    async fn route(&self, account_id: &str) -> Result<ManagedRoute, AdminError>;
}

pub(crate) struct DefaultManagedRoutesService {
    store: Option<Arc<dyn ManagedRouteStore>>,
    sources: Option<Arc<dyn SubscriptionStore>>,
    host: Arc<dyn EngineHost>,
    probe: Arc<dyn ProxyProbe>,
    snapshot: Arc<dyn SnapshotControl>,
    active: Mutex<BTreeSet<String>>,
}
impl DefaultManagedRoutesService {
    pub(crate) fn new(
        store: Option<Arc<dyn ManagedRouteStore>>,
        sources: Option<Arc<dyn SubscriptionStore>>,
        host: Arc<dyn EngineHost>,
        probe: Arc<dyn ProxyProbe>,
        snapshot: Arc<dyn SnapshotControl>,
    ) -> Self {
        Self {
            store,
            sources,
            host,
            probe,
            snapshot,
            active: Mutex::new(BTreeSet::new()),
        }
    }
    fn store(&self) -> Result<&Arc<dyn ManagedRouteStore>, AdminError> {
        self.store.as_ref().ok_or_else(unavailable)
    }
    async fn facts(&self, owner: EngineOwner, node: &RouteNode) -> Result<EngineFacts, AdminError> {
        let source = self
            .sources
            .as_ref()
            .ok_or_else(unavailable)?
            .load(&node.source_id)
            .await
            .map_err(store_error)?;
        let stored = source
            .nodes
            .iter()
            .find(|n| n.id == node.node_id && n.available && n.version_id == node.version_id)
            .ok_or_else(|| AdminError::conflict("节点批准版本已变化，请重新准备候选"))?;
        let version = source
            .versions
            .iter()
            .find(|v| v.id == stored.version_id && v.node_id == stored.id)
            .ok_or_else(unavailable)?;
        Ok(EngineFacts {
            entry_id: owner.entry_id(),
            owner,
            source_id: source.summary.id,
            node_id: stored.id.clone(),
            version_id: version.id.clone(),
            node_name: stored.name.clone(),
            protocol: stored.protocol.clone(),
            connection: version.connection.0.clone(),
        })
    }
    async fn retained_facts(
        &self,
        owner: EngineOwner,
        node: &RouteNode,
    ) -> Result<EngineFacts, AdminError> {
        let source = self
            .sources
            .as_ref()
            .ok_or_else(unavailable)?
            .load(&node.source_id)
            .await
            .map_err(store_error)?;
        let version = source
            .versions
            .iter()
            .find(|version| version.id == node.version_id && version.node_id == node.node_id)
            .ok_or_else(|| AdminError::conflict("已批准的连接历史缺失"))?;
        Ok(EngineFacts {
            entry_id: owner.entry_id(),
            owner,
            source_id: node.source_id.clone(),
            node_id: node.node_id.clone(),
            version_id: node.version_id.clone(),
            node_name: node.node_name.clone(),
            protocol: node.protocol.clone(),
            connection: version.connection.0.clone(),
        })
    }

    async fn receipt(&self, operation: RouteOperation) -> Result<RouteReceipt, AdminError> {
        let Some(route) = self
            .store()?
            .route(&operation.account_id)
            .await
            .map_err(store_error)?
        else {
            let requires_reconciliation = operation.phase != RoutePhase::Failed;
            return Ok(RouteReceipt {
                operation,
                route: None,
                engine: None,
                requires_reconciliation,
            });
        };
        let owner = EngineOwner {
            kind: EngineOwnerKind::Account,
            id: operation.account_id.clone(),
        };
        let engine = self.host.list().await.ok().and_then(|items| {
            items
                .into_iter()
                .find(|item| item.entry_id == owner.entry_id())
        });
        let current_matches = match (&engine, &route.current) {
            (Some(engine), Some(node)) => {
                engine.state == EngineState::Running
                    && engine.node_id == node.node_id
                    && engine.version_id == node.version_id
                    && engine.source_id == node.source_id
            }
            _ => false,
        };
        let requires_reconciliation = operation.phase == RoutePhase::Applying
            || (route.generation > 0
                && (!current_matches || route.account_proxy_id != route.proxy_id));
        Ok(RouteReceipt {
            operation,
            route: Some(route),
            engine: engine.map(Into::into),
            requires_reconciliation,
        })
    }
    async fn republish(
        &self,
        operation: RouteOperation,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError> {
        let owner = EngineOwner {
            kind: EngineOwnerKind::Account,
            id: operation.account_id.clone(),
        };
        let endpoint = self
            .host
            .endpoint(&owner.entry_id())
            .await
            .map_err(engine_error)?;
        let (applied, revision) = self
            .store()?
            .publish(&operation.operation_id, endpoint, context)
            .await
            .map_err(store_error)?;
        publish_committed(self.snapshot.as_ref(), revision).await?;
        self.receipt(applied).await
    }

    async fn apply(
        &self,
        operation: RouteOperation,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError> {
        let owner = EngineOwner {
            kind: EngineOwnerKind::Account,
            id: operation.account_id.clone(),
        };
        let facts = self.facts(owner.clone(), &operation.candidate).await?;
        // 数据库先记录applying，外部进程完成后再提交绑定；中断后按同operation读回
        let engine = self.host.replace(facts).await.map_err(engine_error)?;
        if engine.state != EngineState::Running {
            return Err(unavailable());
        }
        let endpoint = self
            .host
            .endpoint(&owner.entry_id())
            .await
            .map_err(engine_error)?;
        let result = self
            .store()?
            .publish(&operation.operation_id, endpoint, context)
            .await;
        let (applied, revision) = match result {
            Ok(value) => value,
            Err(error) => {
                // 提交响应可能已丢失，读回确认前停止入口使外部状态保持明确
                let observed = self.store()?.operation(&operation.operation_id).await;
                if let Ok(applied) = observed
                    && applied.phase == RoutePhase::Applied
                {
                    return self.republish(applied, context).await;
                }
                let _ = self.host.stop(&owner.entry_id()).await;
                return Err(store_error(error));
            }
        };
        publish_committed(self.snapshot.as_ref(), revision).await?;
        self.receipt(applied).await
    }
}
#[async_trait]
impl ManagedRoutesService for DefaultManagedRoutesService {
    async fn prepare(
        &self,
        command: PrepareRoute,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError> {
        let account = ProviderAccountId::new(command.account_id)
            .map_err(|_| AdminError::invalid("账号标识校验失败"))?;
        let operation_id = uuid::Uuid::parse_str(&command.operation_id)
            .map_err(|_| AdminError::invalid("操作标识需为UUID"))?
            .to_string();
        let source_id = uuid::Uuid::parse_str(&command.source_id)
            .map_err(|_| AdminError::invalid("来源标识需为UUID"))?
            .to_string();
        let node_id = uuid::Uuid::parse_str(&command.node_id)
            .map_err(|_| AdminError::invalid("节点标识需为UUID"))?
            .to_string();
        let _guard = Active::begin(&self.active, account.as_str())?;
        // 相同请求读回既有状态，避免再次建立候选或重复探测
        match self.store()?.operation(&operation_id).await {
            Ok(operation) => {
                if operation.account_id != account.as_str()
                    || operation.expected_generation != command.expected_generation
                    || operation.candidate.source_id != source_id
                    || operation.candidate.node_id != node_id
                {
                    return Err(AdminError::conflict("操作标识已用于另一请求"));
                }
                return self.receipt(operation).await;
            }
            Err(error) if error.kind() == crate::ports::store::AdminStoreErrorKind::NotFound => {}
            Err(error) => return Err(store_error(error)),
        }
        let source = self
            .sources
            .as_ref()
            .ok_or_else(unavailable)?
            .load(&source_id)
            .await
            .map_err(store_error)?;
        let node = source
            .nodes
            .iter()
            .find(|n| n.id == node_id && n.available)
            .ok_or_else(|| AdminError::invalid("请选择当前可用的批准节点"))?;
        let candidate = RouteNode {
            source_id: source.summary.id.clone(),
            node_id: node.id.clone(),
            version_id: node.version_id.clone(),
            node_name: node.name.clone(),
            protocol: node.protocol.clone(),
        };
        let owner = EngineOwner {
            kind: EngineOwnerKind::Reservation,
            id: operation_id.clone(),
        };
        let facts = self.facts(owner.clone(), &candidate).await?;
        let (operation, created) = self
            .store()?
            .begin(
                BeginRoute {
                    operation_id: operation_id.clone(),
                    account_id: account.as_str().to_owned(),
                    expected_generation: command.expected_generation,
                    candidate,
                },
                context,
            )
            .await
            .map_err(store_error)?;
        if !created {
            return self.receipt(operation).await;
        }
        let mut cleanup = CandidateCleanup {
            host: self.host.clone(),
            entry_id: Some(owner.entry_id()),
        };
        let probe_result = async {
            self.host.start(facts).await.map_err(engine_error)?;
            let endpoint = self
                .host
                .endpoint(&owner.entry_id())
                .await
                .map_err(engine_error)?;
            let test = self.probe.test(ProbeRoute::Proxy(&endpoint), false).await;
            if !test.success {
                return Err(AdminError::new(
                    AdminErrorKind::BadGateway,
                    "候选出口连接验证失败，当前入口保持原值",
                ));
            }
            Ok(RouteObservation {
                success: test.success,
                latency_ms: test.latency_ms,
                exit_ip: test.exit_ip,
                exit_ipv4: test.exit_ipv4,
                exit_ipv6: test.exit_ipv6,
            })
        }
        .await;
        let cleanup_result = self.host.discard_candidate(&owner.entry_id()).await;
        if cleanup_result.is_ok() {
            cleanup.entry_id = None;
        }
        match probe_result {
            Ok(observation) => {
                cleanup_result.map_err(engine_error)?;
                let ready = self
                    .store()?
                    .ready(&operation_id, observation)
                    .await
                    .map_err(store_error)?;
                self.receipt(ready).await
            }
            Err(error) => {
                self.store()?
                    .fail(&operation_id, "route_candidate_failed")
                    .await
                    .map_err(store_error)?;
                Err(error)
            }
        }
    }
    async fn approve(
        &self,
        id: &str,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError> {
        let operation = self.store()?.operation(id).await.map_err(store_error)?;
        let _guard = Active::begin(&self.active, &operation.account_id)?;
        if operation.phase == RoutePhase::Applied {
            return self.republish(operation, context).await;
        }
        if operation.phase != RoutePhase::Ready {
            return Err(AdminError::conflict("操作当前需要读回或重新准备候选"));
        }
        self.facts(
            EngineOwner {
                kind: EngineOwnerKind::Account,
                id: operation.account_id.clone(),
            },
            &operation.candidate,
        )
        .await?;
        let claimed = self.store()?.claim(id).await.map_err(store_error)?;
        self.apply(claimed, context).await
    }
    async fn reconcile(
        &self,
        id: &str,
        context: &MutationContext,
    ) -> Result<RouteReceipt, AdminError> {
        let operation = self.store()?.operation(id).await.map_err(store_error)?;
        let _guard = Active::begin(&self.active, &operation.account_id)?;
        match operation.phase {
            RoutePhase::Applying => match self.store()?.claim(id).await {
                Ok(current) => self.apply(current, context).await,
                Err(error)
                    if error.kind() == crate::ports::store::AdminStoreErrorKind::Conflict =>
                {
                    let route = self.route(&operation.account_id).await?;
                    let owner = EngineOwner {
                        kind: EngineOwnerKind::Account,
                        id: operation.account_id.clone(),
                    };
                    let _ = self.host.stop(&owner.entry_id()).await;
                    let restored = if let Some(previous) = route.current {
                        match self.retained_facts(owner, &previous).await {
                            Ok(facts) => self.host.replace(facts).await.is_ok(),
                            Err(_) => false,
                        }
                    } else {
                        true
                    };
                    let failed = self
                        .store()?
                        .fail_with_context(
                            id,
                            if restored {
                                "route_candidate_superseded"
                            } else {
                                "route_restore_required"
                            },
                            Some(context),
                        )
                        .await
                        .map_err(store_error)?;
                    self.receipt(failed).await
                }
                Err(error) => Err(store_error(error)),
            },
            RoutePhase::Applied => {
                let route = self.route(&operation.account_id).await?;
                if route.generation != operation.expected_generation + 1
                    || route.current.as_ref() != Some(&operation.candidate)
                    || route.account_proxy_id != route.proxy_id
                {
                    return Err(AdminError::conflict(
                        "账号或路由已进入后续版本，请读取最新操作",
                    ));
                }
                let facts = self
                    .retained_facts(
                        EngineOwner {
                            kind: EngineOwnerKind::Account,
                            id: operation.account_id.clone(),
                        },
                        &operation.candidate,
                    )
                    .await?;
                self.host.start(facts).await.map_err(engine_error)?;
                self.republish(operation, context).await
            }
            RoutePhase::Preparing => {
                let owner = EngineOwner {
                    kind: EngineOwnerKind::Reservation,
                    id: operation.operation_id.clone(),
                };
                self.host
                    .discard_candidate(&owner.entry_id())
                    .await
                    .map_err(engine_error)?;
                let failed = self
                    .store()?
                    .fail_with_context(id, "route_prepare_interrupted", Some(context))
                    .await
                    .map_err(store_error)?;
                self.receipt(failed).await
            }
            _ => self.receipt(operation).await,
        }
    }
    async fn abort(&self, id: &str, context: &MutationContext) -> Result<RouteReceipt, AdminError> {
        let operation = self.store()?.operation(id).await.map_err(store_error)?;
        let _guard = Active::begin(&self.active, &operation.account_id)?;
        if operation.phase == RoutePhase::Applied {
            return Err(AdminError::conflict(
                "已发布操作保留确认记录，请准备新的换路操作",
            ));
        }
        if operation.phase == RoutePhase::Failed {
            return self.receipt(operation).await;
        }
        let candidate = EngineOwner {
            kind: EngineOwnerKind::Reservation,
            id: operation.operation_id.clone(),
        };
        self.host
            .discard_candidate(&candidate.entry_id())
            .await
            .map_err(engine_error)?;
        let mut restored = true;
        if operation.phase == RoutePhase::Applying {
            let owner = EngineOwner {
                kind: EngineOwnerKind::Account,
                id: operation.account_id.clone(),
            };
            let _ = self.host.stop(&owner.entry_id()).await;
            if let Some(previous) = self.route(&operation.account_id).await?.current {
                restored = match self.retained_facts(owner, &previous).await {
                    Ok(facts) => self.host.replace(facts).await.is_ok(),
                    Err(_) => false,
                };
            }
        }
        let failed = self
            .store()?
            .fail_with_context(
                id,
                if restored {
                    "route_operation_cancelled"
                } else {
                    "route_restore_required"
                },
                Some(context),
            )
            .await
            .map_err(store_error)?;
        self.receipt(failed).await
    }

    async fn operation(&self, id: &str) -> Result<RouteReceipt, AdminError> {
        self.receipt(self.store()?.operation(id).await.map_err(store_error)?)
            .await
    }
    async fn route(&self, account_id: &str) -> Result<ManagedRoute, AdminError> {
        self.store()?
            .route(account_id)
            .await
            .map_err(store_error)?
            .ok_or_else(|| AdminError::new(AdminErrorKind::NotFound, "受管路由记录缺失"))
    }
}
struct Active<'a> {
    entries: &'a Mutex<BTreeSet<String>>,
    id: String,
}
impl<'a> Active<'a> {
    fn begin(entries: &'a Mutex<BTreeSet<String>>, id: &str) -> Result<Self, AdminError> {
        if !entries
            .lock()
            .map_err(|_| unavailable())?
            .insert(id.to_owned())
        {
            return Err(AdminError::conflict("该账号路由操作正在执行"));
        }
        Ok(Self {
            entries,
            id: id.to_owned(),
        })
    }
}
impl Drop for Active<'_> {
    fn drop(&mut self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&self.id);
        }
    }
}
struct CandidateCleanup {
    host: Arc<dyn EngineHost>,
    entry_id: Option<String>,
}
impl Drop for CandidateCleanup {
    fn drop(&mut self) {
        if let Some(id) = self.entry_id.take() {
            let host = self.host.clone();
            tokio::spawn(async move {
                let _ = host.discard_candidate(&id).await;
            });
        }
    }
}
fn unavailable() -> AdminError {
    AdminError::new(AdminErrorKind::Unavailable, "受管路由服务暂时异常")
}
fn store_error(error: AdminStoreError) -> AdminError {
    super::map_store_error(error, "managed route")
}
fn engine_error(error: EngineServiceError) -> AdminError {
    let kind = match error {
        EngineServiceError::PortConflict
        | EngineServiceError::BindingConflict
        | EngineServiceError::Cancelled => AdminErrorKind::Conflict,
        EngineServiceError::Capacity => AdminErrorKind::RateLimited,
        EngineServiceError::NodeUnsupported | EngineServiceError::OwnerInvalid => {
            AdminErrorKind::Invalid
        }
        _ => AdminErrorKind::Unavailable,
    };
    AdminError::new(kind, format!("受管引擎操作失败：{}", error.reason()))
}
