//! 入口登记与每入口监督任务的组合边界

mod artifact;
mod config;
mod entries;
mod observation;
mod runtime;

use async_trait::async_trait;
use config::EngineSettings;
use entries::{EntryRecord, EntryRegistry};
use gateway_admin::{
    model::engines::{EngineFacts, EngineServiceError as Error, EngineSnapshot, EngineState},
    ports::engine::EngineHost,
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
    done: watch::Receiver<bool>,
}
struct Inner {
    registry: Result<EntryRegistry, Error>,
    runs: BTreeMap<String, RunHandle>,
}

/// 全局锁只提交登记和任务句柄，进程等待属于每个入口的监督任务
pub(super) struct EngineSupervisor {
    settings: Result<Arc<EngineSettings>, Error>,
    root: PathBuf,
    inner: Mutex<Inner>,
    capacity: Arc<Semaphore>,
    cancellation: CancellationToken,
    transitions: std::sync::Mutex<std::collections::BTreeSet<String>>,
}

impl EngineSupervisor {
    pub(super) fn new(data: &Path, cancellation: CancellationToken) -> Self {
        let root = data.join("engines");
        let settings = EngineSettings::from_env().map(Arc::new);
        let capacity = Arc::new(Semaphore::new(settings.as_ref().map_or(0, |s| s.capacity)));
        let registry = EntryRegistry::load(&root);
        Self {
            settings,
            root,
            inner: Mutex::new(Inner {
                registry,
                runs: BTreeMap::new(),
            }),
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

impl EngineSupervisor {
    async fn start_entry(
        &self,
        facts: EngineFacts,
        allow_transition: bool,
    ) -> Result<EngineSnapshot, Error> {
        let settings = self.settings()?;
        config::validate_connection(&facts)?;
        let deadline = Instant::now() + settings.start_timeout;
        let (run, receiver) = {
            let mut inner = timeout_at(deadline, self.inner.lock())
                .await
                .map_err(|_| Error::StartTimeout)?;
            if !allow_transition
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
                .filter(|r| !(*r.done.borrow()))
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
            let record = registry.reserve(&facts, settings.port_base, settings.port_span)?;
            if let Some(existing) = active {
                (existing, None)
            } else {
                let permit = permit.ok_or(Error::Capacity)?;
                let mut initial = stopped(&record);
                initial.state = EngineState::Starting;
                let (status, observe) = watch::channel(initial);
                let (done, completed) = watch::channel(false);
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
                    let _ = timeout(
                        runtime::STOP_GRACE + Duration::from_secs(2),
                        wait_done(run.done.clone()),
                    )
                    .await;
                    Err(Error::StartTimeout)
                }
            };
            guard.0 = None;
            result.map(sample)
        } else {
            // 等待同一入口的既有操作，副本请求只观察共享结果
            let mut status = run.status.clone();
            loop {
                if run.cancel.is_cancelled() {
                    return Err(Error::Cancelled);
                }
                let observed = status.borrow().clone();
                match observed.state {
                    EngineState::Running => return Ok(sample(observed)),
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
        self.start_entry(facts, false).await
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
        self.start_entry(facts, true).await
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

    async fn list(&self) -> Result<Vec<EngineSnapshot>, Error> {
        let inner = self.inner.lock().await;
        let registry = inner.registry.as_ref().map_err(|e| *e)?;
        Ok(registry
            .records()
            .iter()
            .map(|record| {
                inner
                    .runs
                    .get(&record.entry_id)
                    .map_or_else(|| stopped(record), |r| sample(r.status.borrow().clone()))
            })
            .collect())
    }
}
async fn wait_done(mut done: watch::Receiver<bool>) -> Result<(), Error> {
    loop {
        if *done.borrow() {
            return Ok(());
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
