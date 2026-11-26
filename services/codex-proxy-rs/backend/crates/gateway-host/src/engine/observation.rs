//! 进程资源采样与有界私有日志

use std::{io::Write as _, os::unix::fs::OpenOptionsExt as _, path::PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt as _};

const LOG_LIMIT: u64 = 1024 * 1024;

/// 每个流只保留一份有限日志，持续排空管道使进程保持运行
pub(super) async fn drain_log<R: AsyncRead + Unpin>(mut reader: R, path: PathBuf) {
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .ok();
    let mut written = 0u64;
    let mut chunk = [0u8; 8192];
    loop {
        let count = match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        if let Some(file) = output.as_mut() {
            let take = count.min(LOG_LIMIT.saturating_sub(written) as usize);
            if file.write_all(&chunk[..take]).is_err() {
                output = None;
            } else {
                written += take as u64;
            }
        }
    }
}

pub(super) fn sample(pid: u32) -> (Option<u64>, Option<f64>) {
    let rss = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("VmRSS:"))
                .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
                .map(|kb| kb * 1024)
        });
    let Some(ticks) = nix::unistd::sysconf(nix::unistd::SysconfVar::CLK_TCK)
        .ok()
        .flatten()
        .filter(|v| *v > 0)
    else {
        return (rss, None);
    };
    let cpu = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|text| {
            let fields: Vec<_> = text.rsplit_once(')')?.1.split_whitespace().collect();
            let user: f64 = fields.get(11)?.parse().ok()?;
            let system: f64 = fields.get(12)?.parse().ok()?;
            Some((user + system) / ticks as f64)
        });
    (rss, cpu)
}
