//! 每个入口的子进程监督、认证就绪与终止回执

use super::{
    artifact,
    config::{self, EngineSettings},
    entries::EntryRecord,
    observation,
};
use gateway_admin::model::engines::{
    EngineFacts, EngineServiceError as Error, EngineSnapshot, EngineState,
};
use gateway_core::lifecycle::CancellationToken;
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use std::{
    io::{Read as _, Write as _},
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
    process::{Child, Command},
    sync::{OwnedSemaphorePermit, oneshot, watch},
    time::{Instant, timeout, timeout_at},
};

pub(super) const STOP_GRACE: Duration = Duration::from_secs(3);

pub(super) struct Launch {
    pub facts: EngineFacts,
    pub record: EntryRecord,
    pub settings: Arc<EngineSettings>,
    pub root: PathBuf,
    pub status: watch::Sender<EngineSnapshot>,
    pub cancel: CancellationToken,
    pub deadline: Instant,
    pub permit: OwnedSemaphorePermit,
    pub ready: oneshot::Sender<Result<EngineSnapshot, Error>>,
    pub done: watch::Sender<bool>,
}

/// 监督任务独占 Child，管理请求取消后仍负责等待进程退出
pub(super) async fn supervise(launch: Launch) {
    let Launch {
        facts,
        mut record,
        settings,
        root,
        status,
        cancel,
        deadline,
        permit,
        ready,
        done,
    } = launch;
    let started = Instant::now();
    let mut ready = Some(ready);
    let prepared = prepare(&facts, &mut record, &settings, &root, &cancel, deadline).await;
    let (mut child, logs) = match prepared {
        Ok(value) => value,
        Err(error) => {
            finish(&status, &cancel, Some(error));
            drop(permit);
            done.send_replace(true);
            if let Some(reply) = ready.take() {
                let _ = reply.send(Err(error));
            }
            return;
        }
    };
    let pid = child.id().unwrap_or_default();
    status.send_modify(|s| {
        s.pid = Some(pid);
        s.started_at = Some(chrono::Utc::now());
    });
    let initialized = tokio::select! {
        biased;
        _=cancel.cancelled()=>Err(Error::Cancelled),
        result=timeout_at(deadline,wait_ready(&mut child,&record))=>result.unwrap_or(Err(Error::StartTimeout)),
    };
    let outcome = match initialized {
        Err(error) => {
            terminate(&mut child, pid).await;
            Some(error)
        }
        Ok(()) => {
            status.send_modify(|s| {
                s.state = EngineState::Running;
                s.cold_start_ms =
                    Some(started.elapsed().as_millis().try_into().unwrap_or(u64::MAX));
                s.last_error = None;
            });
            let sent = ready
                .take()
                .is_some_and(|reply| reply.send(Ok(status.borrow().clone())).is_ok());
            if !sent {
                cancel.cancel();
            }
            tokio::select! {
                biased;
                _=cancel.cancelled()=>{terminate(&mut child,pid).await;None},
                exit=child.wait()=>{
                    status.send_modify(|s|s.last_error=Some(match exit {Ok(value)=>format!("engine exited: {value}"),Err(_)=>"engine wait failed".to_owned()}));
                    Some(Error::ProcessFailed)
                }
            }
        }
    };
    for mut log in logs {
        if timeout(Duration::from_millis(500), &mut log).await.is_err() {
            log.abort();
            let _ = log.await;
        }
    }
    finish(&status, &cancel, outcome);
    drop(permit);
    done.send_replace(true);
    if let Some(reply) = ready.take() {
        let _ = reply.send(Err(outcome.unwrap_or(Error::Cancelled)));
    }
}

fn finish(
    status: &watch::Sender<EngineSnapshot>,
    cancel: &CancellationToken,
    error: Option<Error>,
) {
    status.send_modify(|s| {
        s.state = if cancel.is_cancelled() {
            EngineState::Stopped
        } else {
            EngineState::Failed
        };
        if let Some(error) = error {
            if s.last_error.is_none() {
                s.last_error = Some(error.reason().to_owned());
            }
        } else {
            s.last_error = None;
        }
        s.pid = None;
        s.rss_bytes = None;
        s.cpu_seconds = None;
    });
}

async fn prepare(
    facts: &EngineFacts,
    record: &mut EntryRecord,
    settings: &EngineSettings,
    root: &Path,
    cancel: &CancellationToken,
    deadline: Instant,
) -> Result<(Child, Vec<tokio::task::JoinHandle<()>>), Error> {
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let verified = artifact::verify(&settings.binary).await?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(Error::StartTimeout);
    }
    // 固定端口先成对占位，配置写完后交给同一次启动
    let listener = std::net::TcpListener::bind(("127.0.0.1", record.listen_port))
        .map_err(|_| Error::PortConflict)?;
    let controller = std::net::TcpListener::bind(("127.0.0.1", record.control_port))
        .map_err(|_| Error::PortConflict)?;
    record.control_secret = random_secret()?;
    let directory = root.join(&record.entry_id);
    std::fs::create_dir_all(&directory).map_err(|_| Error::Unavailable)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| Error::Unavailable)?;
    let text = config::render_config(
        facts,
        record.listen_port,
        record.control_port,
        &record.control_secret,
        &record.username,
        &record.password,
    )?;
    let path = directory.join("config.yaml");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|_| Error::Unavailable)?;
    file.write_all(text.as_bytes())
        .map_err(|_| Error::Unavailable)?;
    file.sync_all().map_err(|_| Error::Unavailable)?;
    drop(listener);
    drop(controller);
    let mut child = Command::new(verified.executable())
        .arg("-d")
        .arg(&directory)
        .arg("-f")
        .arg(&path)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &directory)
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| Error::ProcessFailed)?;
    let output = child.stdout.take().ok_or(Error::ProcessFailed)?;
    let error = child.stderr.take().ok_or(Error::ProcessFailed)?;
    let logs = vec![
        tokio::spawn(observation::drain_log(output, directory.join("stdout.log"))),
        tokio::spawn(observation::drain_log(error, directory.join("stderr.log"))),
    ];
    Ok((child, logs))
}

async fn wait_ready(child: &mut Child, record: &EntryRecord) -> Result<(), Error> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_millis(350))
        .build()
        .map_err(|_| Error::Unavailable)?;
    loop {
        if child
            .try_wait()
            .map_err(|_| Error::ProcessFailed)?
            .is_some()
        {
            return Err(Error::ProcessFailed);
        }
        let control = client
            .get(format!("http://127.0.0.1:{}/version", record.control_port))
            .bearer_auth(&record.control_secret)
            .send()
            .await;
        if let Ok(reply) = control {
            if reply.status() == reqwest::StatusCode::UNAUTHORIZED {
                return Err(Error::ProcessFailed);
            }
            if reply.status().is_success() {
                let version = reply
                    .json::<serde_json::Value>()
                    .await
                    .map_err(|_| Error::ProcessFailed)?;
                if version.get("version").and_then(serde_json::Value::as_str)
                    != Some(artifact::EXPECTED_VERSION)
                {
                    return Err(Error::ArtifactIdentity);
                }
                if matches!(
                    timeout(Duration::from_millis(350), authenticate_listener(record)).await,
                    Ok(Ok(()))
                ) {
                    if child
                        .try_wait()
                        .map_err(|_| Error::ProcessFailed)?
                        .is_none()
                    {
                        return Ok(());
                    }
                    return Err(Error::ProcessFailed);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// SOCKS5 认证确认本次入口已使用所属凭据，空认证保持拒绝
async fn authenticate_listener(record: &EntryRecord) -> Result<(), Error> {
    let mut socket = TcpStream::connect(("127.0.0.1", record.listen_port))
        .await
        .map_err(|_| Error::ProcessFailed)?;
    socket
        .write_all(&[5, 1, 2])
        .await
        .map_err(|_| Error::ProcessFailed)?;
    let mut reply = [0u8; 2];
    socket
        .read_exact(&mut reply)
        .await
        .map_err(|_| Error::ProcessFailed)?;
    if reply != [5, 2] {
        return Err(Error::ProcessFailed);
    }
    let mut auth = vec![
        1,
        record
            .username
            .len()
            .try_into()
            .map_err(|_| Error::Configuration)?,
    ];
    auth.extend_from_slice(record.username.as_bytes());
    auth.push(
        record
            .password
            .len()
            .try_into()
            .map_err(|_| Error::Configuration)?,
    );
    auth.extend_from_slice(record.password.as_bytes());
    socket
        .write_all(&auth)
        .await
        .map_err(|_| Error::ProcessFailed)?;
    socket
        .read_exact(&mut reply)
        .await
        .map_err(|_| Error::ProcessFailed)?;
    if reply != [1, 0] {
        return Err(Error::ProcessFailed);
    }
    drop(socket);
    let mut anonymous = TcpStream::connect(("127.0.0.1", record.listen_port))
        .await
        .map_err(|_| Error::ProcessFailed)?;
    anonymous
        .write_all(&[5, 1, 0])
        .await
        .map_err(|_| Error::ProcessFailed)?;
    anonymous
        .read_exact(&mut reply)
        .await
        .map_err(|_| Error::ProcessFailed)?;
    if reply == [5, 0] {
        return Err(Error::ProcessFailed);
    }
    Ok(())
}

/// 直接向受管进程组发送信号，SIGSTOP 场景先继续执行以处理正常终止
async fn terminate(child: &mut Child, pid: u32) {
    if let Ok(raw) = i32::try_from(pid) {
        let group = Pid::from_raw(raw);
        let _ = killpg(group, Signal::SIGTERM);
        let _ = killpg(group, Signal::SIGCONT);
        if timeout(STOP_GRACE, child.wait()).await.is_ok() {
            return;
        }
        let _ = killpg(group, Signal::SIGKILL);
    }
    let _ = child.start_kill();
    let _ = child.wait().await;
}
fn random_secret() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|_| Error::Unavailable)?;
    Ok(hex::encode(bytes))
}
