//! 受管引擎的归属登记、稳定端口与私有认证材料持久化

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{Read as _, Write as _},
    os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _},
    path::{Path, PathBuf},
};

use gateway_admin::model::engines::{EngineFacts, EngineOwner, EngineServiceError};
use serde::{Deserialize, Serialize};

const REGISTRY_VERSION: u32 = 2;
const MAX_REGISTERED_ENTRIES: usize = 256;

/// 旧节点入口仅保留端口占位，新入口按 owner 固定认证与批准版本
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EntryRecord {
    pub(super) entry_id: String,
    pub(super) owner: Option<EngineOwner>,
    pub(super) source_id: String,
    pub(super) node_id: String,
    pub(super) version_id: String,
    pub(super) node_name: String,
    pub(super) protocol: String,
    pub(super) listen_port: u16,
    pub(super) control_port: u16,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) control_secret: String,
}

impl std::fmt::Debug for EntryRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntryRecord")
            .field("entry_id", &self.entry_id)
            .field("owner", &self.owner)
            .field("source_id", &self.source_id)
            .field("node_id", &self.node_id)
            .field("version_id", &self.version_id)
            .field("listen_port", &self.listen_port)
            .field("control_port", &self.control_port)
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .field("control_secret", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEntryRecord {
    entry_id: String,
    source_id: String,
    node_id: String,
    node_name: String,
    protocol: String,
    listen_port: u16,
    control_port: u16,
}

impl From<LegacyEntryRecord> for EntryRecord {
    fn from(record: LegacyEntryRecord) -> Self {
        Self {
            entry_id: record.entry_id,
            owner: None,
            source_id: record.source_id,
            node_id: record.node_id,
            version_id: String::new(),
            node_name: record.node_name,
            protocol: record.protocol,
            listen_port: record.listen_port,
            control_port: record.control_port,
            username: String::new(),
            password: String::new(),
            control_secret: String::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRegistry {
    version: u32,
    entries: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct RegistryDocument<'a> {
    version: u32,
    entries: &'a [EntryRecord],
}

/// 磁盘登记是端口归属的权威，写入成功后才发布内存记录
pub(super) struct EntryRegistry {
    path: PathBuf,
    records: Vec<EntryRecord>,
}

impl EntryRegistry {
    pub(super) fn load(root: &Path) -> Result<Self, EngineServiceError> {
        fs::create_dir_all(root).map_err(|_| EngineServiceError::Registry)?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| EngineServiceError::Registry)?;
        let path = root.join("entries.json");
        let bytes = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                Some(fs::read(&path).map_err(|_| EngineServiceError::Registry)?)
            }
            Ok(_) => return Err(EngineServiceError::Registry),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // 遗留配置证明入口曾存在，要求恢复登记后继续使用原端口
                if has_entry_config(root)? {
                    return Err(EngineServiceError::Registry);
                }
                None
            }
            Err(_) => return Err(EngineServiceError::Registry),
        };
        let (records, rewrite) = match bytes {
            Some(bytes) => {
                let stored: StoredRegistry =
                    serde_json::from_slice(&bytes).map_err(|_| EngineServiceError::Registry)?;
                let records = match stored.version {
                    1 => stored
                        .entries
                        .into_iter()
                        .map(|value| {
                            serde_json::from_value::<LegacyEntryRecord>(value)
                                .map(EntryRecord::from)
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| EngineServiceError::Registry)?,
                    REGISTRY_VERSION => stored
                        .entries
                        .into_iter()
                        .map(serde_json::from_value::<EntryRecord>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| EngineServiceError::Registry)?,
                    _ => return Err(EngineServiceError::Registry),
                };
                (records, stored.version == 1)
            }
            None => (Vec::new(), true),
        };
        validate_records(&records)?;
        let registry = Self { path, records };
        if rewrite {
            registry.persist(&registry.records)?;
        } else {
            fs::set_permissions(&registry.path, fs::Permissions::from_mode(0o600))
                .map_err(|_| EngineServiceError::Registry)?;
        }
        Ok(registry)
    }

    pub(super) fn records(&self) -> &[EntryRecord] {
        &self.records
    }

    /// 重启复用既有绑定与认证；绑定变化交由明确的换路流程处理
    pub(super) fn reserve(
        &mut self,
        facts: &EngineFacts,
        port_base: u16,
        port_span: u16,
    ) -> Result<EntryRecord, EngineServiceError> {
        if facts.owner.clone().normalize().ok().as_ref() != Some(&facts.owner) {
            return Err(EngineServiceError::OwnerInvalid);
        }
        if facts.entry_id != owner_entry_id(&facts.owner) {
            return Err(EngineServiceError::BindingConflict);
        }
        if let Some(record) = self.records.iter().find(|r| r.entry_id == facts.entry_id) {
            if record.owner.as_ref() != Some(&facts.owner)
                || record.source_id != facts.source_id
                || record.node_id != facts.node_id
                || record.version_id != facts.version_id
            {
                return Err(EngineServiceError::BindingConflict);
            }
            return Ok(record.clone());
        }
        if self.records.len() >= MAX_REGISTERED_ENTRIES {
            return Err(EngineServiceError::Capacity);
        }
        if !canonical_uuid(&facts.source_id)
            || !canonical_uuid(&facts.node_id)
            || !canonical_uuid(&facts.version_id)
        {
            return Err(EngineServiceError::Configuration);
        }
        let (listen_port, control_port) = self.allocate(port_base, port_span)?;
        let record = EntryRecord {
            entry_id: facts.entry_id.clone(),
            owner: Some(facts.owner.clone()),
            source_id: facts.source_id.clone(),
            node_id: facts.node_id.clone(),
            version_id: facts.version_id.clone(),
            node_name: facts.node_name.clone(),
            protocol: facts.protocol.clone(),
            listen_port,
            control_port,
            username: format!("engine-{}", random_hex(16)?),
            password: random_hex(32)?,
            control_secret: random_hex(32)?,
        };
        let mut records = self.records.clone();
        records.push(record.clone());
        self.persist(&records)?;
        self.records = records;
        Ok(record)
    }

    /// 发布明确批准的版本，端口与入口认证保持原值
    pub(super) fn rebind(
        &mut self,
        facts: &EngineFacts,
    ) -> Result<EntryRecord, EngineServiceError> {
        let mut records = self.records.clone();
        let Some(record) = records.iter_mut().find(|r| r.entry_id == facts.entry_id) else {
            return Err(EngineServiceError::UnknownEntry);
        };
        if record.owner.as_ref() != Some(&facts.owner) {
            return Err(EngineServiceError::BindingConflict);
        }
        record.source_id = facts.source_id.clone();
        record.node_id = facts.node_id.clone();
        record.version_id = facts.version_id.clone();
        record.node_name = facts.node_name.clone();
        record.protocol = facts.protocol.clone();
        let result = record.clone();
        validate_records(&records)?;
        self.persist(&records)?;
        self.records = records;
        Ok(result)
    }

    /// 候选仅借用预留项入口，验证完成后显式释放端口登记
    pub(super) fn discard_candidate(&mut self, entry_id: &str) -> Result<(), EngineServiceError> {
        if let Some(record) = self.records.iter().find(|r| r.entry_id == entry_id) {
            if record.owner.as_ref().is_none_or(|owner| {
                owner.kind != gateway_admin::model::engines::EngineOwnerKind::Reservation
            }) {
                return Err(EngineServiceError::OwnerInvalid);
            }
        } else {
            return Ok(());
        }
        let records: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.entry_id != entry_id)
            .cloned()
            .collect();
        self.persist(&records)?;
        self.records = records;
        Ok(())
    }

    pub(super) fn discard_authorization(
        &mut self,
        entry_id: &str,
    ) -> Result<(), EngineServiceError> {
        if let Some(record) = self.records.iter().find(|r| r.entry_id == entry_id) {
            if record.owner.as_ref().is_none_or(|owner| {
                owner.kind != gateway_admin::model::engines::EngineOwnerKind::Account
            }) {
                return Err(EngineServiceError::OwnerInvalid);
            }
        } else {
            return Ok(());
        }
        let records: Vec<_> = self
            .records
            .iter()
            .filter(|r| r.entry_id != entry_id)
            .cloned()
            .collect();
        self.persist(&records)?;
        self.records = records;
        Ok(())
    }

    fn allocate(&self, base: u16, span: u16) -> Result<(u16, u16), EngineServiceError> {
        let end = u32::from(base) + u32::from(span);
        if base == 0 || span < 2 || end > u32::from(u16::MAX) + 1 {
            return Err(EngineServiceError::Configuration);
        }
        let reserved: BTreeSet<u16> = self
            .records
            .iter()
            .flat_map(|record| [record.listen_port, record.control_port])
            .collect();
        let mut candidate = u32::from(base);
        while candidate + 1 < end {
            let listen = u16::try_from(candidate).map_err(|_| EngineServiceError::Configuration)?;
            let control =
                u16::try_from(candidate + 1).map_err(|_| EngineServiceError::Configuration)?;
            if !reserved.contains(&listen)
                && !reserved.contains(&control)
                && let Ok(_listener) = std::net::TcpListener::bind(("127.0.0.1", listen))
                && let Ok(_controller) = std::net::TcpListener::bind(("127.0.0.1", control))
            {
                return Ok((listen, control));
            }
            candidate += 2;
        }
        Err(EngineServiceError::PortConflict)
    }

    fn persist(&self, records: &[EntryRecord]) -> Result<(), EngineServiceError> {
        let document = RegistryDocument {
            version: REGISTRY_VERSION,
            entries: records,
        };
        let bytes =
            serde_json::to_vec_pretty(&document).map_err(|_| EngineServiceError::Registry)?;
        let root = self.path.parent().ok_or(EngineServiceError::Registry)?;
        let temporary = root.join(format!(".entries-{}.tmp", random_hex(16)?));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| EngineServiceError::Registry)?;
        let result = file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .and_then(|()| fs::rename(&temporary, &self.path));
        if result.is_err() {
            // 此临时路径由本次 create_new 独占，失败只回收该次写入
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|_| EngineServiceError::Registry)
    }
}

fn has_entry_config(root: &Path) -> Result<bool, EngineServiceError> {
    for entry in fs::read_dir(root).map_err(|_| EngineServiceError::Registry)? {
        let entry = entry.map_err(|_| EngineServiceError::Registry)?;
        if entry
            .file_type()
            .map_err(|_| EngineServiceError::Registry)?
            .is_dir()
        {
            match fs::symlink_metadata(entry.path().join("config.yaml")) {
                Ok(_) => return Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(EngineServiceError::Registry),
            }
        }
    }
    Ok(false)
}

fn validate_records(records: &[EntryRecord]) -> Result<(), EngineServiceError> {
    if records.len() > MAX_REGISTERED_ENTRIES {
        return Err(EngineServiceError::Capacity);
    }
    let mut identities = BTreeSet::new();
    let mut ports = BTreeSet::new();
    for record in records {
        if record.listen_port == 0
            || record.listen_port.checked_add(1) != Some(record.control_port)
            || !identities.insert(&record.entry_id)
            || !ports.insert(record.listen_port)
            || !ports.insert(record.control_port)
        {
            return Err(EngineServiceError::Registry);
        }
        match &record.owner {
            Some(owner) => {
                if owner.clone().normalize().ok().as_ref() != Some(owner)
                    || record.entry_id != owner_entry_id(owner)
                    || !canonical_uuid(&record.source_id)
                    || !canonical_uuid(&record.node_id)
                    || !canonical_uuid(&record.version_id)
                    || record.username.is_empty()
                    || record.password.is_empty()
                    || record.control_secret.is_empty()
                {
                    return Err(EngineServiceError::Registry);
                }
            }
            None => {
                if record.source_id.is_empty()
                    || record.node_id.is_empty()
                    || record.entry_id != format!("{}:{}", record.source_id, record.node_id)
                    || !record.version_id.is_empty()
                    || !record.username.is_empty()
                    || !record.password.is_empty()
                    || !record.control_secret.is_empty()
                {
                    return Err(EngineServiceError::Registry);
                }
            }
        }
    }
    Ok(())
}

fn owner_entry_id(owner: &EngineOwner) -> String {
    owner.entry_id()
}

/// Host 只接收 Admin 已规范化的 UUID，加载持久记录时验证这一合同
fn canonical_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

fn random_hex(length: usize) -> Result<String, EngineServiceError> {
    let mut bytes = vec![0_u8; length];
    fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|_| EngineServiceError::Registry)?;
    Ok(hex::encode(bytes))
}
