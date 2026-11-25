//! 受管 Mihomo 引擎的启动、就绪探测、停止与资源观测
//!
//! 每个入口一个独立进程，监听在本机回环地址；进程随监督器释放而回收

mod config;
mod entries;

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gateway_admin::model::engines::{EngineFacts, EngineServiceError, EngineSnapshot, EngineState};
use gateway_admin::ports::engine::EngineHost;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

use config::EngineSettings;
use entries::{EntryRecord, EntryRegistry};

const STOP_GRACE: std::time::Duration = std::time::Duration::from_secs(5);
const PROBE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const TICKS_PER_SECOND: f64 = 100.0;

/// 单个入口的运行中进程
struct Session {
    child: Child,
    _permit: OwnedSemaphorePermit,
    pid: u32,
    started_at: DateTime<Utc>,
    cold_start_ms: u64,
}

struct Inner {
    registry: EntryRegistry,
    sessions: BTreeMap<String, Session>,
    failures: BTreeMap<String, String>,
}

/// 进程级监督器；配置来自部署环境，入口记录落在运行数据目录
pub struct EngineSupervisor {
    settings: EngineSettings,
    root: PathBuf,
    inner: Mutex<Inner>,
    capacity: Arc<Semaphore>,
}

impl EngineSupervisor {
    #[must_use]
    pub fn new(runtime_data_dir: &Path) -> Self {
        let root = runtime_data_dir.join("engines");
        if let Err(error) = std::fs::create_dir_all(&root) {
            tracing::warn!(target: "gateway_engine", %error, "引擎数据目录创建失败");
        }
        let settings = EngineSettings::from_env();
        let registry = EntryRegistry::load(&root);
        Self {
            capacity: Arc::new(Semaphore::new(settings.capacity)),
            settings,
            root,
            inner: Mutex::new(Inner {
                registry,
                sessions: BTreeMap::new(),
                failures: BTreeMap::new(),
            }),
        }
    }

    /// 进程退出后回收槽位与观测事实；由每次入口操作先执行
    fn sweep(inner: &mut Inner) {
        let mut exited = Vec::new();
        for (entry_id, session) in &mut inner.sessions {
            if let Ok(Some(status)) = session.child.try_wait() {
                exited.push((entry_id.clone(), status.to_string()));
            }
        }
        for (entry_id, status) in exited {
            inner.sessions.remove(&entry_id);
            inner
                .failures
                .insert(entry_id, format!("engine process exited: {status}"));
        }
    }

    fn snapshot(inner: &Inner, record: &EntryRecord) -> EngineSnapshot {
        let session = inner.sessions.get(&record.entry_id);
        let state = if session.is_some() {
            EngineState::Running
        } else if inner.failures.contains_key(&record.entry_id) {
            EngineState::Failed
        } else {
            EngineState::Stopped
        };
        let pid = session.map(|item| item.pid);
        let (rss_bytes, cpu_seconds) = pid.map_or((None, None), sample_process);
        EngineSnapshot {
            entry_id: record.entry_id.clone(),
            source_id: record.source_id.clone(),
            node_id: record.node_id.clone(),
            node_name: record.node_name.clone(),
            protocol: record.protocol.clone(),
            listen_host: "127.0.0.1".to_owned(),
            listen_port: record.listen_port,
            state,
            pid,
            started_at: session.map(|item| item.started_at),
            rss_bytes,
            cpu_seconds,
            cold_start_ms: session.map(|item| item.cold_start_ms),
            last_error: inner.failures.get(&record.entry_id).cloned(),
        }
    }

    fn entry_dir(&self, entry_id: &str) -> PathBuf {
        let name: String = entry_id
            .chars()
            .map(|character| match character {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' => character,
                _ => '_',
            })
            .collect();
        self.root.join(name)
    }

    fn prepare_dir(&self, entry_id: &str) -> Result<PathBuf, EngineServiceError> {
        let directory = self.entry_dir(entry_id);
        std::fs::create_dir_all(&directory).map_err(|_| EngineServiceError::Unavailable)?;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| EngineServiceError::Unavailable)?;
        Ok(directory)
    }

    fn write_config(
        &self,
        directory: &Path,
        facts: &EngineFacts,
        record: &EntryRecord,
    ) -> Result<(), EngineServiceError> {
        let secret = secret();
        let document =
            config::render_config(facts, record.listen_port, record.control_port, &secret)
                .map_err(|_| EngineServiceError::NodeUnsupported)?;
        let path = directory.join("config.yaml");
        std::fs::write(&path, document).map_err(|_| EngineServiceError::Unavailable)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| EngineServiceError::Unavailable)?;
        Ok(())
    }

    async fn wait_ready(
        child: &mut Child,
        listen_port: u16,
        timeout: std::time::Duration,
    ) -> Result<(), EngineServiceError> {
        let deadline = Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => return Err(EngineServiceError::ProcessFailed),
                Ok(None) => {}
            }
            if TcpStream::connect(("127.0.0.1", listen_port)).await.is_ok() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(EngineServiceError::StartTimeout);
            }
            tokio::time::sleep(PROBE_INTERVAL).await;
        }
    }
}

#[async_trait]
impl EngineHost for EngineSupervisor {
    async fn start(&self, facts: EngineFacts) -> Result<EngineSnapshot, EngineServiceError> {
        let mut inner = self.inner.lock().await;
        Self::sweep(&mut inner);
        if inner.sessions.contains_key(&facts.entry_id) {
            let record = inner
                .registry
                .find(&facts.entry_id)
                .cloned()
                .ok_or(EngineServiceError::UnknownEntry)?;
            return Ok(Self::snapshot(&inner, &record));
        }
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|_| EngineServiceError::Capacity)?;
        let record = match inner.registry.find(&facts.entry_id).cloned() {
            Some(record) => record,
            None => {
                let (listen_port, control_port) = inner
                    .registry
                    .allocate(self.settings.port_base, self.settings.port_span)
                    .map_err(|_| EngineServiceError::PortConflict)?;
                let record = EntryRecord {
                    entry_id: facts.entry_id.clone(),
                    source_id: facts.source_id.clone(),
                    node_id: facts.node_id.clone(),
                    node_name: facts.node_name.clone(),
                    protocol: facts.protocol.clone(),
                    listen_port,
                    control_port,
                };
                inner
                    .registry
                    .insert(record.clone())
                    .map_err(|_| EngineServiceError::Unavailable)?;
                record
            }
        };
        if !self.settings.binary.is_file() {
            return Err(EngineServiceError::BinaryMissing);
        }
        // 端口在生成配置前确认可用，占用时停发入口，避免静默换端口
        if std::net::TcpListener::bind(("127.0.0.1", record.listen_port)).is_err() {
            return Err(EngineServiceError::PortConflict);
        }
        let directory = self.prepare_dir(&facts.entry_id)?;
        self.write_config(&directory, &facts, &record)?;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("engine.log"))
            .map_err(|_| EngineServiceError::Unavailable)?;
        let stderr = log
            .try_clone()
            .map_err(|_| EngineServiceError::Unavailable)?;
        let started = Instant::now();
        let mut child = Command::new(&self.settings.binary)
            .arg("-d")
            .arg(&directory)
            .arg("-f")
            .arg(directory.join("config.yaml"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(log))
            .stderr(std::process::Stdio::from(stderr))
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| EngineServiceError::ProcessFailed)?;
        let pid = child.id().ok_or(EngineServiceError::ProcessFailed)?;
        if let Err(error) =
            Self::wait_ready(&mut child, record.listen_port, self.settings.start_timeout).await
        {
            let _ = child.start_kill();
            let _ = child.wait().await;
            inner.failures.insert(
                facts.entry_id.clone(),
                format!("engine start failed: {}", error.reason()),
            );
            return Err(error);
        }
        let cold_start_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        inner.failures.remove(&facts.entry_id);
        inner.sessions.insert(
            facts.entry_id.clone(),
            Session {
                child,
                _permit: permit,
                pid,
                started_at: Utc::now(),
                cold_start_ms,
            },
        );
        Ok(Self::snapshot(&inner, &record))
    }

    async fn stop(&self, entry_id: &str) -> Result<EngineSnapshot, EngineServiceError> {
        let mut inner = self.inner.lock().await;
        Self::sweep(&mut inner);
        let record = inner
            .registry
            .find(entry_id)
            .cloned()
            .ok_or(EngineServiceError::UnknownEntry)?;
        if let Some(mut session) = inner.sessions.remove(entry_id) {
            terminate(&mut session.child, session.pid).await;
        }
        inner.failures.remove(entry_id);
        Ok(Self::snapshot(&inner, &record))
    }

    async fn list(&self) -> Result<Vec<EngineSnapshot>, EngineServiceError> {
        let mut inner = self.inner.lock().await;
        Self::sweep(&mut inner);
        let records = inner.registry.records().to_vec();
        Ok(records
            .iter()
            .map(|record| Self::snapshot(&inner, record))
            .collect())
    }
}

/// 先请求正常退出，超过宽限期后强制回收
async fn terminate(child: &mut Child, pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .await;
    let deadline = Instant::now() + STOP_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(PROBE_INTERVAL).await;
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}

/// 从 /proc 读取常驻内存与累计 CPU 时间
fn sample_process(pid: u32) -> (Option<u64>, Option<f64>) {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok();
    let rss_bytes = status.as_deref().and_then(|text| {
        text.lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|value| value.trim().trim_end_matches(" kB").parse::<u64>().ok())
            .map(|kilobytes| kilobytes * 1024)
    });
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
    let cpu_seconds = stat.as_deref().and_then(|text| {
        let tail = text.rsplit_once(')')?.1;
        let fields: Vec<&str> = tail.split_whitespace().collect();
        let user: f64 = fields.get(11)?.parse().ok()?;
        let system: f64 = fields.get(12)?.parse().ok()?;
        Some((user + system) / TICKS_PER_SECOND)
    });
    (rss_bytes, cpu_seconds)
}

/// 本机控制面密钥；只用于同进程启动的参数传递
fn secret() -> String {
    let mut bytes = [0_u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut bytes))
        .is_err()
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        bytes.copy_from_slice(&nanos.to_le_bytes());
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
