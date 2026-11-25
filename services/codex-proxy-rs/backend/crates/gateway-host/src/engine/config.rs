//! 由批准节点生成最小 Mihomo 配置并解析引擎运行参数

use std::{path::PathBuf, time::Duration};

use gateway_admin::model::engines::EngineFacts;

const BINARY_ENV: &str = "CPR_MIHOMO_BIN";
const CAPACITY_ENV: &str = "CPR_ENGINE_MAX";
const PORT_BASE_ENV: &str = "CPR_ENGINE_PORT_BASE";
const PORT_SPAN_ENV: &str = "CPR_ENGINE_PORT_SPAN";
const START_TIMEOUT_ENV: &str = "CPR_ENGINE_START_TIMEOUT_MS";
const DEFAULT_BINARY: &str = "/app/bin/mihomo";
const DEFAULT_CAPACITY: usize = 4;
const DEFAULT_PORT_BASE: u16 = 24000;
const DEFAULT_PORT_SPAN: u16 = 200;
const DEFAULT_START_TIMEOUT_MS: u64 = 15_000;

/// 引擎监督参数；全部来自部署环境，运行期只读
#[derive(Debug, Clone)]
pub(super) struct EngineSettings {
    pub binary: PathBuf,
    pub capacity: usize,
    pub port_base: u16,
    pub port_span: u16,
    pub start_timeout: Duration,
}

impl EngineSettings {
    pub(super) fn from_env() -> Self {
        Self {
            binary: std::env::var_os(BINARY_ENV)
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY)),
            capacity: parse_env(CAPACITY_ENV, DEFAULT_CAPACITY),
            port_base: parse_env(PORT_BASE_ENV, DEFAULT_PORT_BASE),
            port_span: parse_env(PORT_SPAN_ENV, DEFAULT_PORT_SPAN),
            start_timeout: Duration::from_millis(parse_env(
                START_TIMEOUT_ENV,
                DEFAULT_START_TIMEOUT_MS,
            )),
        }
    }
}

fn parse_env<T: std::str::FromStr>(name: &str, fallback: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

/// 生成只指向单一批准出站的最小配置
///
/// 配置只包含入口监听、该节点与默认规则；不引入 provider、脚本或外部控制面之外的路径
pub(super) fn render_config(
    facts: &EngineFacts,
    listen_port: u16,
    control_port: u16,
    secret: &str,
) -> Result<String, &'static str> {
    let connection = facts.connection.as_object().ok_or("shape")?;
    let server = connection
        .get("server")
        .and_then(serde_json::Value::as_str)
        .ok_or("server")?;
    let port = connection
        .get("port")
        .and_then(|value| match value {
            serde_json::Value::Number(number) => number.as_u64(),
            serde_json::Value::String(text) => text.parse().ok(),
            _ => None,
        })
        .ok_or("port")?;
    let (proxy_type, secure) = match facts.protocol.as_str() {
        "http" => ("http", false),
        "https" => ("http", true),
        "socks5" | "socks5h" => ("socks5", false),
        _ => return Err("protocol"),
    };
    let mut document = String::new();
    document.push_str("log-level: warning\nallow-lan: false\nmode: rule\nipv6: false\n");
    document.push_str(&format!(
        "external-controller: 127.0.0.1:{control_port}\nsecret: {}\n",
        quote(secret)
    ));
    document.push_str("proxies:\n  - name: node\n");
    document.push_str(&format!("    type: {proxy_type}\n"));
    document.push_str(&format!("    server: {}\n", quote(server)));
    document.push_str(&format!("    port: {port}\n"));
    if let Some(username) = connection
        .get("username")
        .and_then(serde_json::Value::as_str)
    {
        document.push_str(&format!("    username: {}\n", quote(username)));
    }
    if let Some(password) = connection
        .get("password")
        .and_then(serde_json::Value::as_str)
    {
        document.push_str(&format!("    password: {}\n", quote(password)));
    }
    if secure {
        document.push_str("    tls: true\n");
    }
    document.push_str("listeners:\n  - name: entry\n    type: mixed\n    listen: 127.0.0.1\n");
    document.push_str(&format!("    port: {listen_port}\n"));
    // 全部流量交给唯一批准节点，不保留 DIRECT 回退
    document.push_str("rules:\n  - MATCH,node\n");
    Ok(document)
}

/// JSON 字符串同时是合法 YAML 标量，避免手写转义规则
fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}
