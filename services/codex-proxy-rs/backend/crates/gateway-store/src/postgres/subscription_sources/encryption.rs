//! 部署专用密钥的延迟加载及订阅文档的认证加密

use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use gateway_admin::model::subscription_sources::SourceDocument;
use ring::{
    aead::{self, Aad, LessSafeKey, Nonce, UnboundKey},
    rand::{SecureRandom, SystemRandom},
};
use tokio::sync::OnceCell;
use uuid::Uuid;

use crate::{StoreBackend, StoreError, StoreResult};

const HEADER: &[u8] = b"CPRSS\x01";
const PURPOSE: &[u8] = b"xinapi:subscription-source:document:v1\0";
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;

pub(super) struct DocumentEncryption {
    key_path: PathBuf,
    key: OnceCell<LessSafeKey>,
}

impl DocumentEncryption {
    pub(super) fn new(key_path: PathBuf) -> Self {
        Self {
            key_path,
            key: OnceCell::new(),
        }
    }

    async fn key(&self, create: bool) -> StoreResult<&LessSafeKey> {
        self.key
            .get_or_try_init(|| async {
                let path = self.key_path.clone();
                tokio::task::spawn_blocking(move || load_or_create_key(&path, create))
                    .await
                    .map_err(|source| {
                        unavailable("load subscription deployment key").with_source(source)
                    })?
            })
            .await
    }

    pub(super) async fn encrypt(
        &self,
        id: Uuid,
        document: &SourceDocument,
        create_key: bool,
    ) -> StoreResult<Vec<u8>> {
        let mut plaintext =
            serde_json::to_vec(document).map_err(|_| invalid("订阅来源文档序列化失败"))?;
        if plaintext.len() > MAX_DOCUMENT_BYTES - HEADER.len() - NONCE_LEN - TAG_LEN {
            return Err(invalid("订阅来源加密文档上限为 32 MiB"));
        }
        let key = self.key(create_key).await?;
        let mut nonce = [0; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| unavailable("generate subscription document nonce"))?;
        key.seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad(id)),
            &mut plaintext,
        )
        .map_err(|_| unavailable("encrypt subscription document"))?;
        let mut envelope = Vec::with_capacity(HEADER.len() + NONCE_LEN + plaintext.len());
        envelope.extend_from_slice(HEADER);
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&plaintext);
        Ok(envelope)
    }

    pub(super) async fn decrypt(&self, id: Uuid, envelope: &[u8]) -> StoreResult<SourceDocument> {
        let prefix_len = HEADER.len() + NONCE_LEN;
        if envelope.len() > MAX_DOCUMENT_BYTES
            || envelope.len() < prefix_len + TAG_LEN
            || !envelope.starts_with(HEADER)
        {
            return Err(invalid("订阅来源加密文档格式校验失败"));
        }
        let nonce: [u8; NONCE_LEN] = envelope[HEADER.len()..prefix_len]
            .try_into()
            .map_err(|_| invalid("订阅来源加密随机值格式校验失败"))?;
        let mut ciphertext = envelope[prefix_len..].to_vec();
        let plaintext = self
            .key(false)
            .await?
            .open_in_place(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(aad(id)),
                &mut ciphertext,
            )
            .map_err(|_| unavailable("authenticate stored subscription document"))?;
        serde_json::from_slice(plaintext).map_err(|_| invalid("订阅来源文档解析失败"))
    }
}

fn aad(id: Uuid) -> Vec<u8> {
    let mut value = Vec::with_capacity(PURPOSE.len() + 16);
    value.extend_from_slice(PURPOSE);
    value.extend_from_slice(id.as_bytes());
    value
}

fn load_or_create_key(path: &Path, create: bool) -> StoreResult<LessSafeKey> {
    match File::open(path) {
        Ok(file) => return read_key(file),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(unavailable("open subscription deployment key").with_source(source));
        }
    }
    if !create {
        return Err(unavailable("subscription deployment key is missing"));
    }
    // 完整写入临时文件后原子发布，多个仓库实例共同采用首个成功发布的密钥
    let temporary_path = path.with_file_name(format!(".subscription-key-{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary_path)
        .map_err(|source| unavailable("create subscription deployment key").with_source(source))?;
    let result = (|| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|source| {
                    unavailable("set subscription deployment key permissions").with_source(source)
                })?;
        }
        let mut bytes = [0; KEY_LEN];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| unavailable("generate subscription deployment key"))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| {
                unavailable("persist subscription deployment key").with_source(source)
            })?;
        match std::fs::hard_link(&temporary_path, path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    let directory = path
                        .parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                        .unwrap_or(Path::new("."));
                    File::open(directory)
                        .and_then(|directory| directory.sync_all())
                        .map_err(|source| {
                            unavailable("persist subscription key directory").with_source(source)
                        })?;
                }
                make_key(&bytes)
            }
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = File::open(path).map_err(|source| {
                    unavailable("open subscription deployment key").with_source(source)
                })?;
                read_key(existing)
            }
            Err(source) => {
                Err(unavailable("publish subscription deployment key").with_source(source))
            }
        }
    })();
    drop(file);
    match std::fs::remove_file(&temporary_path) {
        Ok(()) => result,
        Err(cleanup) => Err(match result {
            Err(error) => error.with_cleanup(cleanup),
            Ok(_) => unavailable("remove subscription key temporary file").with_source(cleanup),
        }),
    }
}

fn read_key(mut file: File) -> StoreResult<LessSafeKey> {
    let metadata = file
        .metadata()
        .map_err(|source| unavailable("inspect subscription deployment key").with_source(source))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o777 != 0o600 {
            return Err(unavailable(
                "subscription deployment key requires mode 0600",
            ));
        }
    }
    if !metadata.is_file() || metadata.len() != KEY_LEN as u64 {
        return Err(unavailable(
            "subscription deployment key requires exactly 32 bytes",
        ));
    }
    let mut bytes = [0; KEY_LEN];
    file.read_exact(&mut bytes)
        .map_err(|source| unavailable("read subscription deployment key").with_source(source))?;
    make_key(&bytes)
}

fn make_key(bytes: &[u8; KEY_LEN]) -> StoreResult<LessSafeKey> {
    UnboundKey::new(&aead::AES_256_GCM, bytes)
        .map(LessSafeKey::new)
        .map_err(|_| unavailable("initialize subscription document encryption"))
}

pub(super) fn invalid(message: &str) -> StoreError {
    StoreError::InvalidData {
        entity: "subscription source",
        message: message.to_owned(),
        source: None,
    }
}

fn unavailable(message: &str) -> StoreError {
    StoreError::Unavailable {
        backend: StoreBackend::PostgreSql,
        message: message.to_owned(),
        source: None,
    }
}
