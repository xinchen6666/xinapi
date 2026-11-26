//! 专用出口授权的预留、单次完成与只读回执服务

use super::CredentialsService;
use crate::{
    model::{
        AdminError, AdminErrorKind, MutationActor, MutationContext,
        engines::{EngineFacts, EngineOwner, EngineOwnerKind},
        managed_routes::{RouteNode, RouteObservation},
        oauth_operations::{OAuthOperation, OAuthOperationView, StartOAuthOperation},
        provider_credentials::{
            AuthorizationMutationTarget, AuthorizationOwnerBinding, AuthorizationReceiptKey,
            CompleteAuthorization, PendingAuthorizationMutation,
        },
    },
    ports::{
        engine::EngineHost,
        managed_routes::ManagedRouteStore,
        oauth_operations::OAuthOperationStore,
        proxy::{ProbeRoute, ProxyProbe},
        subscription_sources::SubscriptionStore,
    },
};
use chrono::{Duration, Utc};
use gateway_core::{
    account::{ManagedEgressRequirement, ProviderAccountId},
    routing::ProviderKind,
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

pub(super) struct ManagedAuthorizationPorts {
    pub(super) store: Arc<dyn OAuthOperationStore>,
    sources: Arc<dyn SubscriptionStore>,
    routes: Arc<dyn ManagedRouteStore>,
    host: Arc<dyn EngineHost>,
    probe: Arc<dyn ProxyProbe>,
    active: Mutex<BTreeSet<uuid::Uuid>>,
}

impl ManagedAuthorizationPorts {
    fn try_activate(&self, id: &str) -> Result<Option<ActiveAuthorization<'_>>, AdminError> {
        let id = uuid::Uuid::parse_str(id).map_err(|_| conflict())?;
        let mut active = self.active.lock().map_err(|_| unavailable())?;
        Ok(active.insert(id).then(|| ActiveAuthorization {
            active: &self.active,
            id,
        }))
    }

    async fn verify_engine_start(
        &self,
        operation: &OAuthOperation,
        actor: &MutationActor,
        phase: &str,
    ) -> Result<(), AdminError> {
        let current = self
            .store
            .get(&operation.request.operation_id, actor)
            .await
            .map_err(error)?
            .ok_or_else(conflict)?;
        if current.phase != phase
            || current.expires_at <= Utc::now()
            || current.planned_account_id != operation.planned_account_id
        {
            return Err(conflict());
        }
        Ok(())
    }
}

/// 同一授权的启机、交换和回收共享活动标记，请求取消时随作用域释放
struct ActiveAuthorization<'a> {
    active: &'a Mutex<BTreeSet<uuid::Uuid>>,
    id: uuid::Uuid,
}

impl Drop for ActiveAuthorization<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(&self.id);
        }
    }
}

fn unavailable() -> AdminError {
    AdminError::new(AdminErrorKind::Unavailable, "授权操作服务暂时失联")
}
fn conflict() -> AdminError {
    AdminError::conflict("授权归属或出口条件已变化，请读取当前回执")
}
fn busy() -> AdminError {
    AdminError::conflict("授权操作正在执行，请读取当前回执")
}
fn operation_id(id: &str) -> Result<String, AdminError> {
    uuid::Uuid::parse_str(id)
        .map(|id| id.to_string())
        .map_err(|_| AdminError::invalid("操作标识需为UUID"))
}
fn error(e: crate::ports::store::AdminStoreError) -> AdminError {
    super::super::map_store_error(e, "oauth operation")
}
impl CredentialsService {
    #[must_use]
    pub fn with_managed_authorization(
        mut self,
        store: Option<Arc<dyn OAuthOperationStore>>,
        sources: Option<Arc<dyn SubscriptionStore>>,
        routes: Option<Arc<dyn ManagedRouteStore>>,
        host: Arc<dyn EngineHost>,
        probe: Arc<dyn ProxyProbe>,
    ) -> Self {
        self.managed = store
            .zip(sources)
            .zip(routes)
            .map(|((store, sources), routes)| ManagedAuthorizationPorts {
                store,
                sources,
                routes,
                host,
                probe,
                active: Mutex::new(BTreeSet::new()),
            });
        self
    }
    fn managed(&self) -> Result<&ManagedAuthorizationPorts, AdminError> {
        self.managed.as_ref().ok_or_else(unavailable)
    }
    pub async fn oauth_operation(
        &self,
        id: &str,
        context: &MutationContext,
    ) -> Result<OAuthOperationView, AdminError> {
        let id = operation_id(id)?;
        Ok(self
            .managed()?
            .store
            .get(&id, &context.actor)
            .await
            .map_err(error)?
            .map_or_else(|| OAuthOperationView::unknown(id), |op| op.view(false)))
    }
    pub async fn start_oauth_operation(
        &self,
        mut request: StartOAuthOperation,
        context: &MutationContext,
    ) -> Result<OAuthOperationView, AdminError> {
        let ports = self.managed()?;
        request.operation_id = operation_id(&request.operation_id)?;
        if !matches!(request.provider.as_str(), "openai" | "xai")
            || request.name.trim().is_empty()
            || request.name.chars().count() > 100
            || request.name.chars().any(char::is_control)
        {
            return Err(AdminError::invalid("服务商与授权名称需为有效值"));
        }
        request.name = request.name.trim().to_owned();
        let active = ports.try_activate(&request.operation_id)?;
        if let Some(existing) = ports
            .store
            .get(&request.operation_id, &context.actor)
            .await
            .map_err(error)?
        {
            if existing.request != request {
                return Err(conflict());
            }
            return Ok(existing.view(true));
        }
        let Some(_active) = active else {
            return Ok(OAuthOperationView::unknown(request.operation_id));
        };
        let kind = ProviderKind::new(request.provider.clone()).map_err(|_| conflict())?;
        self.for_provider(&kind)?;
        let (planned_account_id, node, proxy_id, proxy_revision, generation, endpoint) =
            if let Some(account_id) = &request.account_id {
                if request.source_id.is_some() || request.node_id.is_some() {
                    return Err(AdminError::invalid("重新授权沿用账号当前托管路由"));
                }
                let account_id =
                    ProviderAccountId::new(account_id.clone()).map_err(|_| conflict())?;
                let details = super::super::required_credential(
                    self.accounts.as_ref(),
                    &kind,
                    &account_id,
                    "oauth operation",
                )
                .await?;
                let route = ports
                    .routes
                    .route(account_id.as_str())
                    .await
                    .map_err(error)?
                    .ok_or_else(conflict)?;
                let proxy_id = route
                    .proxy_id
                    .filter(|id| Some(id) == route.account_proxy_id.as_ref())
                    .ok_or_else(conflict)?;
                let proxy = self.proxies.get(&proxy_id).await.map_err(error)?;
                if !proxy.enabled
                    || !details.credential.enabled
                    || route.applying_operation_id.is_some()
                    || route.generation == 0
                {
                    return Err(conflict());
                }
                (
                    account_id.to_string(),
                    route.current.ok_or_else(conflict)?,
                    proxy_id,
                    Some(proxy.revision.get()),
                    route.generation,
                    Some(proxy.proxy.expose_url().to_owned()),
                )
            } else {
                let source_id = operation_id(
                    request
                        .source_id
                        .as_deref()
                        .ok_or_else(|| AdminError::invalid("新授权请选择已批准来源与节点"))?,
                )?;
                let node_id = operation_id(request.node_id.as_deref().ok_or_else(conflict)?)?;
                request.source_id = Some(source_id.clone());
                request.node_id = Some(node_id.clone());
                let source = ports.sources.load(&source_id).await.map_err(error)?;
                let node = source
                    .nodes
                    .iter()
                    .find(|n| n.id == node_id && n.available)
                    .ok_or_else(conflict)?;
                let node = RouteNode {
                    source_id,
                    node_id: node.id.clone(),
                    version_id: node.version_id.clone(),
                    node_name: node.name.clone(),
                    protocol: node.protocol.clone(),
                };
                (
                    format!("acct_{}", uuid::Uuid::new_v4().simple()),
                    node,
                    format!(
                        "proxy_managed_{}",
                        uuid::Uuid::parse_str(&request.operation_id)
                            .map_err(|_| conflict())?
                            .simple()
                    ),
                    None,
                    1,
                    None,
                )
            };
        let source = ports.sources.load(&node.source_id).await.map_err(error)?;
        let operation = OAuthOperation {
            request,
            owner: context.actor.clone(),
            planned_account_id,
            node,
            observation: None,
            source_revision: source.summary.revision,
            proxy_id,
            proxy_revision,
            route_generation: generation,
            endpoint,
            phase: "preparing".into(),
            flow_id: None,
            authorization_url: None,
            expires_at: Utc::now() + Duration::minutes(10),
            reason: None,
            receipt_confirmed: false,
        };
        let (mut operation, created) = ports.store.begin(operation).await.map_err(error)?;
        if !created {
            return Ok(operation.view(true));
        }
        let result = async {
            let facts = self.authorization_facts(&operation).await?;
            ports
                .verify_engine_start(&operation, &context.actor, "preparing")
                .await?;
            ports.host.start(facts).await.map_err(|_| unavailable())?;
            let owner = EngineOwner {
                kind: EngineOwnerKind::Account,
                id: operation.planned_account_id.clone(),
            };
            let endpoint = ports
                .host
                .endpoint(&owner.entry_id())
                .await
                .map_err(|_| unavailable())?;
            if operation
                .endpoint
                .as_deref()
                .is_some_and(|frozen| frozen != endpoint.expose_url())
            {
                return Err(conflict());
            }
            ports
                .store
                .endpoint(
                    &operation.request.operation_id,
                    &context.actor,
                    endpoint.expose_url(),
                )
                .await
                .map_err(error)?;
            operation.endpoint = Some(endpoint.expose_url().into());
            let proxy = endpoint
                .with_route_generation(operation.route_generation)
                .with_managed_requirement(ManagedEgressRequirement::Authorization {
                    operation_id: operation.request.operation_id.clone(),
                    account_id: operation.planned_account_id.clone(),
                });
            let probe = ports.probe.test(ProbeRoute::Proxy(&proxy), false).await;
            if !probe.success {
                return Err(AdminError::new(
                    AdminErrorKind::BadGateway,
                    "授权预留出口验证失败",
                ));
            }
            let observation = RouteObservation {
                success: probe.success,
                latency_ms: probe.latency_ms,
                exit_ip: probe.exit_ip,
                exit_ipv4: probe.exit_ipv4,
                exit_ipv6: probe.exit_ipv6,
            };
            let target = if operation.is_new() {
                AuthorizationMutationTarget::Create {
                    name: operation.request.name.clone(),
                }
            } else {
                AuthorizationMutationTarget::Reauthorize {
                    account_id: ProviderAccountId::new(operation.planned_account_id.clone())
                        .map_err(|_| conflict())?,
                }
            };
            let pending = PendingAuthorizationMutation::new(
                kind.clone(),
                target,
                AuthorizationOwnerBinding::from_context(context),
            )
            .with_outbound_proxy(Some(proxy))
            .with_outbound_proxy_id(Some(operation.proxy_id.clone()))
            .with_managed_authorization(operation.request.operation_id.clone());
            let scope = self.for_provider(&kind)?;
            let started = scope
                .provider
                .start_authorization(pending)
                .await
                .map_err(|e| super::super::map_provider_error(e, "oauth operation"))?;
            let ready = ports
                .store
                .ready(
                    &operation.request.operation_id,
                    &context.actor,
                    started,
                    observation,
                )
                .await
                .map_err(error)?;
            Ok(ready.view(true))
        }
        .await;
        if result.is_err() {
            let _ = ports
                .store
                .fail(
                    &operation.request.operation_id,
                    &context.actor,
                    "start_failed",
                )
                .await;
            self.reclaim_authorization(&operation, &context.actor)
                .await?;
        }
        result
    }
    async fn authorization_facts(
        &self,
        operation: &OAuthOperation,
    ) -> Result<EngineFacts, AdminError> {
        let source = self
            .managed()?
            .sources
            .load(&operation.node.source_id)
            .await
            .map_err(error)?;
        if source.summary.revision != operation.source_revision
            || !source.nodes.iter().any(|n| {
                n.id == operation.node.node_id
                    && n.available
                    && (!operation.is_new() || n.version_id == operation.node.version_id)
            })
        {
            return Err(conflict());
        }
        let version = source
            .versions
            .iter()
            .find(|v| v.node_id == operation.node.node_id && v.id == operation.node.version_id)
            .ok_or_else(conflict)?;
        let owner = EngineOwner {
            kind: EngineOwnerKind::Account,
            id: operation.planned_account_id.clone(),
        };
        Ok(EngineFacts {
            entry_id: owner.entry_id(),
            owner,
            source_id: operation.node.source_id.clone(),
            node_id: operation.node.node_id.clone(),
            version_id: operation.node.version_id.clone(),
            node_name: operation.node.node_name.clone(),
            protocol: operation.node.protocol.clone(),
            connection: version.connection.0.clone(),
        })
    }
    pub async fn complete_oauth_operation(
        &self,
        id: &str,
        callback_url: String,
        context: &MutationContext,
    ) -> Result<OAuthOperationView, AdminError> {
        let id = operation_id(id)?;
        let ports = self.managed()?;
        let Some(_active) = ports.try_activate(&id)? else {
            return self.oauth_operation(&id, context).await;
        };
        let Some(operation) = ports.store.get(&id, &context.actor).await.map_err(error)? else {
            return Ok(OAuthOperationView::unknown(id));
        };
        if operation.phase != "pending" || operation.expires_at <= Utc::now() {
            return Ok(operation.view(false));
        }
        if callback_url.is_empty()
            || callback_url.len() > 16384
            || callback_url.chars().any(char::is_control)
        {
            return Err(AdminError::invalid("回调地址需为有效单行文本"));
        }
        let Some(operation) = ports
            .store
            .claim(&id, &context.actor)
            .await
            .map_err(error)?
        else {
            return self.oauth_operation(&id, context).await;
        };
        let result = async {
            let facts = self.authorization_facts(&operation).await?;
            ports
                .verify_engine_start(&operation, &context.actor, "completing")
                .await?;
            ports.host.start(facts).await.map_err(|_| unavailable())?;
            let owner = EngineOwner {
                kind: EngineOwnerKind::Account,
                id: operation.planned_account_id.clone(),
            };
            let endpoint = ports
                .host
                .endpoint(&owner.entry_id())
                .await
                .map_err(|_| unavailable())?;
            if operation.endpoint.as_deref() != Some(endpoint.expose_url()) {
                return Err(conflict());
            }
            let kind =
                ProviderKind::new(operation.request.provider.clone()).map_err(|_| conflict())?;
            let scope = self.for_provider(&kind)?;
            let flow_id = operation.flow_id.ok_or_else(unavailable)?;
            let key = AuthorizationReceiptKey::new(kind.clone(), &flow_id, context)?;
            let prepared = scope
                .provider
                .complete_authorization(CompleteAuthorization {
                    settings: None,
                    context: context.clone(),
                    flow_id,
                    callback_url,
                })
                .await
                .map_err(|e| super::super::map_provider_error(e, "oauth operation"))?;
            if prepared.pending.managed_authorization() != Some(id.as_str()) {
                prepared.abort().await?;
                return Err(conflict());
            }
            let prepared = super::super::validate_authorization_commit(
                &kind,
                context,
                prepared,
                "oauth operation",
            )
            .await?;
            let outcome = super::super::commit_authorization(
                self.accounts.as_ref(),
                prepared,
                key,
                None,
                context,
                "oauth operation",
            )
            .await?;
            self.publish_authorization(&scope.provider, outcome, &context.request_id)
                .await?;
            Ok(())
        }
        .await;
        if result.is_err() {
            let _ = ports
                .store
                .fail(&id, &context.actor, "completion_unknown")
                .await;
            if let Some(current) = ports.store.get(&id, &context.actor).await.map_err(error)? {
                self.reclaim_authorization(&current, &context.actor).await?;
            }
        }
        self.oauth_operation(&id, context).await
    }
    pub async fn cancel_oauth_operation(
        &self,
        id: &str,
        context: &MutationContext,
    ) -> Result<OAuthOperationView, AdminError> {
        let id = operation_id(id)?;
        let ports = self.managed()?;
        let _active = ports.try_activate(&id)?.ok_or_else(busy)?;
        if ports
            .store
            .get(&id, &context.actor)
            .await
            .map_err(error)?
            .is_none()
        {
            return Ok(OAuthOperationView::unknown(id));
        }
        let operation = ports
            .store
            .cancel(&id, &context.actor)
            .await
            .map_err(error)?;
        self.reclaim_authorization(&operation, &context.actor)
            .await?;
        Ok(operation.view(false))
    }
    pub async fn reclaim_expired_oauth_reservation(
        &self,
        id: &str,
        context: &MutationContext,
    ) -> Result<String, AdminError> {
        let id = operation_id(id)?;
        let ports = self.managed()?;
        let _active = ports.try_activate(&id)?.ok_or_else(busy)?;
        let operation = ports
            .store
            .cancel_expired_reservation(&id, context)
            .await
            .map_err(error)?;
        self.reclaim_authorization(&operation, &operation.owner)
            .await?;
        Ok(id)
    }

    async fn reclaim_authorization(
        &self,
        operation: &OAuthOperation,
        owner: &MutationActor,
    ) -> Result<(), AdminError> {
        let ports = self.managed()?;
        if ports
            .store
            .reclaimable(&operation.request.operation_id, owner)
            .await
            .map_err(error)?
        {
            let owner = EngineOwner {
                kind: EngineOwnerKind::Account,
                id: operation.planned_account_id.clone(),
            };
            ports
                .host
                .discard_authorization(&owner.entry_id())
                .await
                .map_err(|_| unavailable())?;
        }
        Ok(())
    }
}
