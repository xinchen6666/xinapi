//! 受管 Mihomo 引擎的入口记录、端口分配与持久化
//!
//! 记录只保存稳定入口所需的事实；连接内容每次启动按当前批准版本重写

use std::{collections::BTreeSet, io::Write as _, path::Path};

use serde::{Deserialize, Serialize};

/// 稳定入口：同一来源节点在重启后复用同一监听端口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EntryRecord {
    pub entry_id: String,
    pub source_id: String,
    pub node_id: String,
    pub node_name: String,
    pub protocol: String,
    pub listen_port: u16,
    pub control_port: u16,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RecordFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    entries: Vec<EntryRecord>,
}

/// 入口记录文件的读写与端口挑选
pub(super) struct EntryRegistry {
    path: std::path::PathBuf,
    records: Vec<EntryRecord>,
}

impl EntryRegistry {
    pub(super) fn load(root: &Path) -> Self {
        let path = root.join("entries.json");
        let records = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<RecordFile>(&bytes).ok())
            .map(|file| file.entries)
            .unwrap_or_default();
        Self { path, records }
    }

    pub(super) fn records(&self) -> &[EntryRecord] {
        &self.records
    }

    pub(super) fn find(&self, entry_id: &str) -> Option<&EntryRecord> {
        self.records.iter().find(|item| item.entry_id == entry_id)
    }

    /// 已有入口复用原端口；新入口在配置范围内挑一对相邻可用端口
    pub(super) fn allocate(
        &mut self,
        port_base: u16,
        port_span: u16,
    ) -> Result<(u16, u16), &'static str> {
        let reserved: BTreeSet<u16> = self
            .records
            .iter()
            .flat_map(|item| [item.listen_port, item.control_port])
            .collect();
        let end = u32::from(port_base) + u32::from(port_span);
        let mut candidate = u32::from(port_base);
        while candidate + 1 < end {
            let listen = u16::try_from(candidate).map_err(|_| "port")?;
            let control = u16::try_from(candidate + 1).map_err(|_| "port")?;
            if !reserved.contains(&listen)
                && !reserved.contains(&control)
                && available(listen)
                && available(control)
            {
                return Ok((listen, control));
            }
            candidate += 2;
        }
        Err("exhausted")
    }

    pub(super) fn insert(&mut self, record: EntryRecord) -> Result<(), &'static str> {
        self.records.push(record);
        self.persist()
    }

    fn persist(&self) -> Result<(), &'static str> {
        let document = RecordFile {
            version: 1,
            entries: self.records.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&document).map_err(|_| "encode")?;
        let temporary = self.path.with_extension("json.tmp");
        {
            let mut file = std::fs::File::create(&temporary).map_err(|_| "write")?;
            file.write_all(&bytes).map_err(|_| "write")?;
            file.sync_all().map_err(|_| "write")?;
        }
        std::fs::rename(&temporary, &self.path).map_err(|_| "rename")
    }
}

fn available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}
