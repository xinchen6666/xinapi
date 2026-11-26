//! 固定 Mihomo 制品的摘要与平台准入，文件句柄固定实际执行对象

use gateway_admin::model::engines::EngineServiceError;
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::Read as _,
    os::fd::AsRawFd as _,
    path::{Path, PathBuf},
};

pub(super) const EXPECTED_VERSION: &str = "v1.19.33";
// 官方amd64-compatible发布包解压后的ELF；固定摘要同时确定版本与平台
const EXPECTED_SHA256: &str = "817f50b3ac6a8b1c4360c5cf2961906057f3a883d970ebbcb0028a01632b620f";
const EXPECTED_SIZE: u64 = 62_079_102;

/// 调用方持有句柄直至spawn返回，部署只读挂载保护制品内容
pub(super) struct VerifiedArtifact {
    file: File,
}
impl VerifiedArtifact {
    pub(super) fn executable(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }
}

/// 摘要确定可信发布身份，正式进程启动后再用认证controller核对版本
pub(super) async fn verify(binary: &Path) -> Result<VerifiedArtifact, EngineServiceError> {
    if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
        return Err(EngineServiceError::ArtifactIdentity);
    }
    let path = binary.to_owned();
    tokio::task::spawn_blocking(move || verify_file(&path))
        .await
        .map_err(|_| EngineServiceError::ArtifactIdentity)?
}
fn verify_file(binary: &Path) -> Result<VerifiedArtifact, EngineServiceError> {
    let mut file = File::open(binary).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => EngineServiceError::BinaryMissing,
        _ => EngineServiceError::ArtifactIdentity,
    })?;
    let metadata = file
        .metadata()
        .map_err(|_| EngineServiceError::ArtifactIdentity)?;
    if !metadata.is_file() || metadata.len() != EXPECTED_SIZE {
        return Err(EngineServiceError::ArtifactIdentity);
    }
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut total = 0u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| EngineServiceError::ArtifactIdentity)?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > EXPECTED_SIZE {
            return Err(EngineServiceError::ArtifactIdentity);
        }
        digest.update(&buffer[..count]);
    }
    if total != EXPECTED_SIZE || hex::encode(digest.finalize()) != EXPECTED_SHA256 {
        return Err(EngineServiceError::ArtifactIdentity);
    }
    Ok(VerifiedArtifact { file })
}
