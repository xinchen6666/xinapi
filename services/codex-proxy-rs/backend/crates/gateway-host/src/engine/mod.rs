//! 入口登记与每入口监督任务的组合边界

pub(super) mod admission;
mod artifact;
mod config;
mod entries;
mod observation;
mod runtime;

use async_trait::async_trait;
use config::EngineSettings;
use entries::{EntryRecord, EntryRegistry};
use gateway_admin::{
    model::{
        engines::{
            EngineCapacity, EngineFacts, EngineOwnerKind, EngineServiceError as Error,
            EngineSnapshot, EngineState,
        },
        managed_routes::{RouteNode, RouteObservation},
    },
    ports::{
        engine::EngineHost,
        proxy::{ProbeRoute, ProxyProbe},
    },
};
use gateway_core::lifecycle::CancellationToken;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{Mutex, Semaphore, oneshot, watch},
    time::{Instant, timeout, timeout_at},
};

#[derive(Clone)]
struct RunHandle {
    cancel: CancellationToken,
    status: watch::Receiver<EngineSnapshot>,
    done: watch::Receiver<Option<Result<(), Error>>>,
}
struct Inner {
    registry: Result<EntryRegistry, Error>,
    runs: BTreeMap<String, RunHandle>,
}

/// 全局锁只提交登记和任务句柄，进程等待属于每个入口的监督任务
pub(super) struct EngineSupervisor {
    settings: Result<Arc<EngineSettings>, Error>,
    root: PathBuf,
    inner: Arc<Mutex<Inner>>,
    capacity: Arc<Semaphore>,
    cancellation: CancellationToken,
    transitions: std::sync::Mutex<std::collections::BTreeSet<String>>,
}

impl EngineSupervisor {
    pub(super) fn new(data: &Path, cancellation: CancellationToken) -> Self {
        let root = data.join("engines");
        let settings = EngineSettings::from_env().map(Arc::new);
        let capacity = Arc::new(Semaphore::new(settings.as_ref().map_or(0, |s| s.capacity)));
        let registry = EntryRegistry::load(&root).and_then(|registry| {
            let tests = root.join("node-tests");
            // 进程启动时回收上一实例遗留的临时连接配置
            match std::fs::symlink_metadata(&tests) {
                Ok(info) if info.is_dir() => {
                    std::fs::remove_dir_all(&tests).map_err(|_| Error::Registry)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(Error::Registry),
            }
            Ok(registry)
        });
        Self {
            settings,
            root,
            inner: Arc::new(Mutex::new(Inner {
                registry,
                runs: BTreeMap::new(),
            })),
            capacity,
            cancellation,
            transitions: std::sync::Mutex::new(std::collections::BTreeSet::new()),
        }
    }
    pub(super) async fn shutdown(&self) {
        self.cancellation.cancel();
        let inner = self.inner.lock().await;
        let completions = futures::future::join_all(inner.runs.values().map(|run| {
            let done = run.done.clone();
            async move {
                let _ = wait_done(done).await;
            }
        }));
        drop(inner);
        completions.await;
    }
    pub(super) async fn authorize_egress(
        &self,
        target: &gateway_core::account::ManagedEgressTarget,
        proxy: &gateway_core::account::OutboundProxy,
    ) -> Result<(), gateway_core::account::ManagedEgressDenied> {
        use gateway_core::account::ManagedEgressDenied as Denied;
        if self.cancellation.is_cancelled() {
            return Err(Denied);
        }
        let inner = self.inner.lock().await;
        if self
            .transitions
            .lock()
            .map_err(|_| Denied)?
            .contains(&target.entry_id)
        {
            return Err(Denied);
        }
        let record = inner
            .registry
            .as_ref()
            .map_err(|_| Denied)?
            .records()
            .iter()
            .find(|r| r.entry_id == target.entry_id)
            .ok_or(Denied)?;
        let run = inner.runs.get(&target.entry_id).ok_or(Denied)?;
        let view = run.status.borrow();
        if run.cancel.is_cancelled()
            || run.done.borrow().is_some()
            || view.state != EngineState::Running
            || record.source_id != target.source_id
            || record.node_id != target.node_id
            || record.version_id != target.version_id
        {
            return Err(Denied);
        }
        let endpoint = format!(
            "http://{}:{}@127.0.0.1:{}/",
            record.username, record.password, record.listen_port
        );
        if proxy.expose_url() != endpoint {
            return Err(Denied);
        }
        let pid = view
            .pid
            .and_then(|pid| i32::try_from(pid).ok())
            .ok_or(Denied)?;
        nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None).map_err(|_| Denied)?;
        Ok(())
    }

    fn settings(&self) -> Result<Arc<EngineSettings>, Error> {
        self.settings.as_ref().cloned().map_err(|e| *e)
    }
}
impl Drop for EngineSupervisor {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

/// 初始请求被取消时发信号，独立监督任务继续执行回收
struct CancelOnDrop(Option<CancellationToken>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(token) = &self.0 {
            token.cancel();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StartMode {
    Stable,
    Replace,
    NodeTest,
}

impl EngineSupervisor {
    async fn start_entry(
        &self,
        facts: EngineFacts,
        mode: StartMode,
    ) -> Result<(EngineSnapshot, RunHandle), Error> {
        let settings = self.settings()?;
        config::validate_connection(&facts)?;
        let deadline = Instant::now() + settings.start_timeout;
        let temporary = mode == StartMode::NodeTest;
        let (run, receiver) = {
            let mut inner = timeout_at(deadline, self.inner.lock())
                .await
                .map_err(|_| Error::StartTimeout)?;
            if mode != StartMode::Replace
                && self
                    .transitions
                    .lock()
                    .map_err(|_| Error::Unavailable)?
                    .contains(&facts.entry_id)
            {
                return Err(Error::BindingConflict);
            }
            if self.cancellation.is_cancelled() {
                return Err(Error::ShuttingDown);
            }
            let active = inner
                .runs
                .get(&facts.entry_id)
                .filter(|r| r.done.borrow().is_none())
                .cloned();
            let permit = if active.is_some() {
                None
            } else {
                Some(
                    self.capacity
                        .clone()
                        .try_acquire_owned()
                        .map_err(|_| Error::Capacity)?,
                )
            };
            let registry = inner.registry.as_mut().map_err(|e| *e)?;
            if !temporary && registry.is_temporary(&facts.entry_id) {
                return Err(Error::BindingConflict);
            }
            let record =
                registry.reserve(&facts, settings.port_base, settings.port_span, temporary)?;
            if let Some(existing) = active {
                (existing, None)
            } else {
                let permit = permit.ok_or(Error::Capacity)?;
                let mut initial = stopped(&record);
                initial.state = EngineState::Starting;
                initial.occupies_slot = true;
                let (status, observe) = watch::channel(initial);
                let (done, completed) = watch::channel(None);
                let cancel = self.cancellation.child_token();
                let run = RunHandle {
                    cancel: cancel.clone(),
                    status: observe,
                    done: completed,
                };
                let (send, receive) = oneshot::channel();
                tokio::spawn(runtime::supervise(runtime::Launch {
                    facts,
                    record: record.clone(),
                    settings: settings.clone(),
                    root: self.root.clone(),
                    status,
                    cancel,
                    deadline,
                    permit,
                    ready: send,
                    done,
                    inner: self.inner.clone(),
                    temporary,
                }));
                inner.runs.insert(record.entry_id, run.clone());
                (run, Some(receive))
            }
        };
        if let Some(receiver) = receiver {
            let mut guard = CancelOnDrop(Some(run.cancel.clone()));
            let result = match timeout_at(deadline, receiver).await {
                Ok(Ok(result)) => result,
                Ok(Err(_)) => Err(Error::ProcessFailed),
                Err(_) => {
                    run.cancel.cancel();
                    let _ = timeout(runtime::CLEANUP_WAIT, wait_done(run.done.clone())).await;
                    Err(Error::StartTimeout)
                }
            };
            guard.0 = None;
            result.map(|observed| (sample(observed), run))
        } else {
            // 等待同一入口的既有操作，副本请求只观察共享结果
            let mut status = run.status.clone();
            loop {
                if run.cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let observed = status.borrow().clone();
                match observed.state {
                    EngineState::Running => return Ok((sample(observed), run)),
                    EngineState::Starting => {}
                    EngineState::Stopped => return Err(Error::Cancelled),
                    EngineState::Failed => return Err(Error::ProcessFailed),
                }
                timeout_at(deadline, status.changed())
                    .await
                    .map_err(|_| Error::StartTimeout)?
                    .map_err(|_| Error::ProcessFailed)?;
                if self.cancellation.is_cancelled() {
                    return Err(Error::ShuttingDown);
                }
            }
        }
    }
}

#[async_trait]
impl EngineHost for EngineSupervisor {
    async fn start(&self, facts: EngineFacts) -> Result<EngineSnapshot, Error> {
        self.start_entry(facts, StartMode::Stable)
            .await
            .map(|(snapshot, _)| snapshot)
    }

    async fn test_node<'a>(
        &self,
        facts: EngineFacts,
        probe: &dyn ProxyProbe,
        validate: futures::future::BoxFuture<'a, Result<(), Error>>,
    ) -> Result<RouteObservation, Error> {
        if facts.owner.kind != EngineOwnerKind::Reservation {
            return Err(Error::OwnerInvalid);
        }
        let deadline = Instant::now() + Duration::from_secs(58);
        let entry_id = facts.entry_id.clone();
        let (_, run) = timeout_at(deadline, self.start_entry(facts, StartMode::NodeTest))
            .await
            .map_err(|_| Error::NodeTestTimeout)??;
        let guard = CancelOnDrop(Some(run.cancel.clone()));
        let result = tokio::select! {
            biased;
            _ = run.cancel.cancelled() => Err(Error::Cancelled),
            result = timeout_at(deadline, async {
                let endpoint = self.endpoint(&entry_id).await?;
                validate.await?;
                let result = probe.test(ProbeRoute::Proxy(&endpoint), false).await;
                Ok(RouteObservation {
                    success: result.success,
                    latency_ms: result.latency_ms,
                    exit_ip: result.exit_ip,
                    exit_ipv4: result.exit_ipv4,
                    exit_ipv6: result.exit_ipv6,
                })
            }) => result.unwrap_or(Err(Error::NodeTestTimeout)),
        };
        drop(guard);
        let cleanup = timeout(runtime::CLEANUP_WAIT, wait_done(run.done)).await;
        match cleanup {
            Ok(Ok(())) => result,
            Ok(Err(error)) => Err(error),
            Err(_) => Err(Error::Unavailable),
        }
    }

    async fn stop_account(
        &self,
        entry_id: &str,
        node: &RouteNode,
    ) -> Result<EngineSnapshot, Error> {
        let _transition = Transition::begin(&self.transitions, entry_id)?;
        {
            let inner = self.inner.lock().await;
            let record = inner
                .registry
                .as_ref()
                .map_err(|error| *error)?
                .records()
                .iter()
                .find(|record| record.entry_id == entry_id)
                .ok_or(Error::UnknownEntry)?;
            if !record
                .owner
                .as_ref()
                .is_some_and(|owner| owner.kind == EngineOwnerKind::Account)
                || record.source_id != node.source_id
                || record.node_id != node.node_id
                || record.version_id != node.version_id
            {
                return Err(Error::StopForbidden);
            }
        }
        self.stop(entry_id).await
    }
    async fn stop(&self, entry_id: &str) -> Result<EngineSnapshot, Error> {
        let (record, run) = {
            let inner = self.inner.lock().await;
            let registry = inner.registry.as_ref().map_err(|e| *e)?;
            let record = registry
                .records()
                .iter()
                .find(|r| r.entry_id == entry_id)
                .cloned()
                .ok_or(Error::UnknownEntry)?;
            (record, inner.runs.get(entry_id).cloned())
        };
        if let Some(run) = run {
            run.cancel.cancel();
            timeout(
                runtime::STOP_GRACE + Duration::from_secs(2),
                wait_done(run.done.clone()),
            )
            .await
            .map_err(|_| Error::Unavailable)??;
            let mut inner = self.inner.lock().await;
            if inner
                .runs
                .get(entry_id)
                .is_some_and(|current| current.done.same_channel(&run.done))
            {
                inner.runs.remove(entry_id);
            }
            Ok(stopped(&record))
        } else {
            Ok(stopped(&record))
        }
    }
    async fn endpoint(
        &self,
        entry_id: &str,
    ) -> Result<gateway_core::account::OutboundProxy, Error> {
        let inner = self.inner.lock().await;
        let record = inner
            .registry
            .as_ref()
            .map_err(|e| *e)?
            .records()
            .iter()
            .find(|r| r.entry_id == entry_id)
            .ok_or(Error::UnknownEntry)?;
        if record.owner.is_none() {
            return Err(Error::OwnerInvalid);
        }
        gateway_core::account::OutboundProxy::parse(&format!(
            "http://{}:{}@127.0.0.1:{}",
            record.username, record.password, record.listen_port
        ))
        .map(|proxy| {
            proxy.with_managed_requirement(
                gateway_core::account::ManagedEgressRequirement::Reservation {
                    entry_id: record.entry_id.clone(),
                    source_id: record.source_id.clone(),
                    node_id: record.node_id.clone(),
                    version_id: record.version_id.clone(),
                },
            )
        })
        .map_err(|_| Error::Configuration)
    }

    async fn replace(&self, facts: EngineFacts) -> Result<EngineSnapshot, Error> {
        config::validate_connection(&facts)?;
        let entry_id = facts.entry_id.clone();
        let _transition = Transition::begin(&self.transitions, &entry_id)?;
        let exists = {
            let inner = self.inner.lock().await;
            inner
                .registry
                .as_ref()
                .map_err(|e| *e)?
                .records()
                .iter()
                .any(|r| r.entry_id == entry_id)
        };
        if exists {
            self.stop(&entry_id).await?;
            let mut inner = self.inner.lock().await;
            inner.registry.as_mut().map_err(|e| *e)?.rebind(&facts)?;
        }
        self.start_entry(facts, StartMode::Replace)
            .await
            .map(|(snapshot, _)| snapshot)
    }

    async fn discard_candidate(&self, entry_id: &str) -> Result<(), Error> {
        let _transition = Transition::begin(&self.transitions, entry_id)?;
        match self.stop(entry_id).await {
            Ok(_) | Err(Error::UnknownEntry) => {}
            Err(error) => return Err(error),
        }
        let mut inner = self.inner.lock().await;
        inner
            .registry
            .as_mut()
            .map_err(|e| *e)?
            .discard_candidate(entry_id)?;
        drop(inner);
        let directory = self.root.join(entry_id);
        if directory.exists() {
            let root = self.root.canonicalize().map_err(|_| Error::Unavailable)?;
            let target = directory.canonicalize().map_err(|_| Error::Unavailable)?;
            if target.parent() != Some(root.as_path()) || !entry_id.starts_with("reservation-") {
                return Err(Error::OwnerInvalid);
            }
            std::fs::remove_dir_all(target).map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }

    async fn discard_authorization(&self, entry_id: &str) -> Result<(), Error> {
        let _transition = Transition::begin(&self.transitions, entry_id)?;
        match self.stop(entry_id).await {
            Ok(_) | Err(Error::UnknownEntry) => {}
            Err(error) => return Err(error),
        }
        let mut inner = self.inner.lock().await;
        inner
            .registry
            .as_mut()
            .map_err(|e| *e)?
            .discard_authorization(entry_id)?;
        drop(inner);
        let directory = self.root.join(entry_id);
        if directory.exists() {
            let root = self.root.canonicalize().map_err(|_| Error::Unavailable)?;
            let target = directory.canonicalize().map_err(|_| Error::Unavailable)?;
            if target.parent() != Some(root.as_path()) || !entry_id.starts_with("account-") {
                return Err(Error::OwnerInvalid);
            }
            std::fs::remove_dir_all(target).map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<EngineSnapshot>, Error> {
        self.inventory().await.map(|(entries, _)| entries)
    }

    async fn inventory(&self) -> Result<(Vec<EngineSnapshot>, EngineCapacity), Error> {
        let settings = self.settings()?;
        let inner = self.inner.lock().await;
        let registry = inner.registry.as_ref().map_err(|e| *e)?;
        let available = self.capacity.available_permits();
        let entries = registry
            .records()
            .iter()
            .map(|record| {
                inner
                    .runs
                    .get(&record.entry_id)
                    .map_or_else(|| stopped(record), |r| sample(r.status.borrow().clone()))
            })
            .collect();
        Ok((
            entries,
            EngineCapacity {
                limit: settings.capacity,
                used: settings.capacity.saturating_sub(available),
                available,
            },
        ))
    }
}
async fn wait_done(mut done: watch::Receiver<Option<Result<(), Error>>>) -> Result<(), Error> {
    loop {
        if let Some(result) = *done.borrow() {
            return result;
        }
        done.changed().await.map_err(|_| Error::ProcessFailed)?;
    }
}
fn sample(mut snapshot: EngineSnapshot) -> EngineSnapshot {
    if let Some(pid) = snapshot.pid {
        let (rss, cpu) = observation::sample(pid);
        snapshot.rss_bytes = rss;
        snapshot.cpu_seconds = cpu;
    }
    snapshot
}
fn stopped(record: &EntryRecord) -> EngineSnapshot {
    EngineSnapshot {
        entry_id: record.entry_id.clone(),
        owner: record.owner.clone(),
        source_id: record.source_id.clone(),
        node_id: record.node_id.clone(),
        version_id: record.version_id.clone(),
        node_name: record.node_name.clone(),
        protocol: record.protocol.clone(),
        listen_host: "127.0.0.1".into(),
        listen_port: record.listen_port,
        state: EngineState::Stopped,
        pid: None,
        started_at: None,
        rss_bytes: None,
        cpu_seconds: None,
        cold_start_ms: None,
        last_error: None,
        occupies_slot: false,
    }
}

/// 切换只占用所属入口的标识，取消时也会释放对后续操作的阻挡
struct Transition<'a> {
    entries: &'a std::sync::Mutex<std::collections::BTreeSet<String>>,
    id: String,
}
impl<'a> Transition<'a> {
    fn begin(
        entries: &'a std::sync::Mutex<std::collections::BTreeSet<String>>,
        id: &str,
    ) -> Result<Self, Error> {
        if !entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .insert(id.to_owned())
        {
            return Err(Error::BindingConflict);
        }
        Ok(Self {
            entries,
            id: id.to_owned(),
        })
    }
}
impl Drop for Transition<'_> {
    fn drop(&mut self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&self.id);
        }
    }
}
