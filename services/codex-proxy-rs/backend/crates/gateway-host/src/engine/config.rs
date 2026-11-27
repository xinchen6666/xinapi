//! 校验引擎部署参数，并把已批准连接生成带独立入口认证的固定配置

use std::{env, path::PathBuf, str::FromStr, time::Duration};

use gateway_admin::model::engines::{EngineFacts, EngineServiceError};
use serde_json::{Map, Value, json};

const DEFAULT_BINARY: &str = "/app/bin/mihomo";
const DEFAULT_CAPACITY: usize = 2;
const DEFAULT_PORT_BASE: u16 = 24000;
const DEFAULT_PORT_SPAN: u16 = 200;
const DEFAULT_START_TIMEOUT_MS: u64 = 15_000;

#[derive(Debug, Clone)]
pub(super) struct EngineSettings {
    pub binary: PathBuf,
    pub capacity: usize,
    pub port_base: u16,
    pub port_span: u16,
    pub start_timeout: Duration,
}

impl EngineSettings {
    pub(super) fn from_env() -> Result<Self, EngineServiceError> {
        let binary = env::var_os("CPR_MIHOMO_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_BINARY));
        let capacity = parse_env("CPR_ENGINE_MAX", DEFAULT_CAPACITY)?;
        let port_base = parse_env("CPR_ENGINE_PORT_BASE", DEFAULT_PORT_BASE)?;
        let port_span = parse_env("CPR_ENGINE_PORT_SPAN", DEFAULT_PORT_SPAN)?;
        let timeout_ms = parse_env("CPR_ENGINE_START_TIMEOUT_MS", DEFAULT_START_TIMEOUT_MS)?;
        if !binary.is_absolute()
            || !(1..=4).contains(&capacity)
            || port_base < 1024
            || port_span < 2
            || port_span % 2 != 0
            || usize::from(port_span) < capacity * 2
            || u32::from(port_base) + u32::from(port_span) > 65536
            || !(1000..=30_000).contains(&timeout_ms)
        {
            return Err(EngineServiceError::Configuration);
        }
        Ok(Self {
            binary,
            capacity,
            port_base,
            port_span,
            start_timeout: Duration::from_millis(timeout_ms),
        })
    }
}

fn parse_env<T: FromStr>(name: &str, fallback: T) -> Result<T, EngineServiceError> {
    match env::var(name) {
        Ok(value) => value.parse().map_err(|_| EngineServiceError::Configuration),
        Err(env::VarError::NotPresent) => Ok(fallback),
        Err(env::VarError::NotUnicode(_)) => Err(EngineServiceError::Configuration),
    }
}

/// 受管节点按 v1.19.33 官方出站合同生成，入口始终固定到单个批准连接
pub(super) fn render_config(
    facts: &EngineFacts,
    listen_port: u16,
    control_port: u16,
    controller_secret: &str,
    username: &str,
    password: &str,
) -> Result<String, EngineServiceError> {
    if listen_port < 1024
        || listen_port.checked_add(1) != Some(control_port)
        || controller_secret.is_empty()
        || !valid_listener_credential(username)
        || username.contains(':')
        || !valid_listener_credential(password)
    {
        return Err(EngineServiceError::Configuration);
    }
    let proxy = render_proxy(facts)?;
    // users 拥有独立认证材料；空 skip-auth-prefixes 强制回环连接也完成认证
    // MATCH 与 listener.proxy 都固定到本次批准节点
    let config = json!({
        "log-level": "warning",
        "allow-lan": false,
        "bind-address": "127.0.0.1",
        "mode": "rule",
        "ipv6": true,
        "find-process-mode": "off",
        "external-controller": format!("127.0.0.1:{control_port}"),
        "secret": controller_secret,
        "skip-auth-prefixes": [],
        "dns": {"enable": false, "use-hosts": false, "use-system-hosts": false},
        "hosts": {},
        "sniffer": {"enable": false},
        "proxies": [proxy],
        "listeners": [{
            "name": "entry",
            "type": "mixed",
            "listen": "127.0.0.1",
            "port": listen_port,
            "proxy": "node",
            "udp": false,
            "users": [{"username": username, "password": password}]
        }],
        "rules": ["MATCH,node"]
    });
    serde_json::to_string_pretty(&config).map_err(|_| EngineServiceError::Configuration)
}

fn valid_listener_credential(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}

pub(super) fn validate_connection(facts: &EngineFacts) -> Result<(), EngineServiceError> {
    render_proxy(facts).map(|_| ())
}

fn render_proxy(facts: &EngineFacts) -> Result<Value, EngineServiceError> {
    if matches!(facts.protocol.as_str(), "ss" | "vmess" | "vless" | "trojan") {
        return super::protocols::render_proxy(facts);
    }
    let input = facts
        .connection
        .as_object()
        .ok_or(EngineServiceError::NodeUnsupported)?;
    let protocol = match facts.protocol.as_str() {
        "http" | "https" => "http",
        "socks5" | "socks5h" => "socks5",
        _ => return Err(EngineServiceError::NodeUnsupported),
    };
    let type_name = input
        .get("type")
        .and_then(Value::as_str)
        .ok_or(EngineServiceError::NodeUnsupported)?;
    if !matches!(
        (protocol, type_name),
        ("http", "http" | "https") | ("socks5", "socks5" | "socks5h")
    ) {
        return Err(EngineServiceError::NodeUnsupported);
    }
    let server = required_string(input, "server")?;
    if server.is_empty() || server.chars().any(char::is_control) {
        return Err(EngineServiceError::NodeUnsupported);
    }
    let port = input
        .get("port")
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|port| (1..=65535).contains(port))
        .ok_or(EngineServiceError::NodeUnsupported)?;
    let mut proxy = Map::new();
    proxy.insert("name".into(), json!("node"));
    proxy.insert("type".into(), json!(protocol));
    proxy.insert("server".into(), json!(server));
    proxy.insert("port".into(), json!(port));

    // SOCKS TCP 的域名由上游编码并交给代理解析，显式 remote 才进入此执行路径
    // 该版本内建 localhost hosts 映射，保留上游的这一特殊名称行为
    if protocol == "socks5" && input.get("dns-mode").and_then(Value::as_str) != Some("remote") {
        return Err(EngineServiceError::NodeUnsupported);
    }
    for (key, value) in input {
        match key.as_str() {
            "type" | "server" | "port" => {}
            "dns-mode" if protocol == "socks5" => {}
            "username" | "password" => {
                string_value(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "tls" | "tfo" | "mptcp" | "skip-cert-verify" => {
                value.as_bool().ok_or(EngineServiceError::NodeUnsupported)?;
                proxy.insert(key.clone(), value.clone());
            }
            "sni" if protocol == "http" => {
                string_value(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "name-cert-verify" | "fingerprint" => {
                string_value(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "certfingerprint" => {
                string_value(value)?;
                if input.get("fingerprint").is_some_and(|other| other != value) {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert("fingerprint".into(), value.clone());
            }
            "certificate" => {
                if !string_value(value)?
                    .trim()
                    .starts_with("-----BEGIN CERTIFICATE-----")
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "private-key" => {
                let pem = string_value(value)?.trim();
                if ![
                    "-----BEGIN PRIVATE KEY-----",
                    "-----BEGIN RSA PRIVATE KEY-----",
                    "-----BEGIN EC PRIVATE KEY-----",
                ]
                .iter()
                .any(|header| pem.starts_with(header))
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "headers" if protocol == "http" => {
                validate_headers(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "udp" if protocol == "socks5" && value == &Value::Bool(false) => {
                proxy.insert(key.clone(), value.clone());
            }
            _ => return Err(EngineServiceError::NodeUnsupported),
        }
    }
    let tls = proxy.get("tls").and_then(Value::as_bool).unwrap_or(false);
    if (facts.protocol == "https" || type_name == "https") && !tls {
        return Err(EngineServiceError::NodeUnsupported);
    }
    let tls_fields = [
        "sni",
        "skip-cert-verify",
        "name-cert-verify",
        "fingerprint",
        "certificate",
        "private-key",
    ];
    if !tls && tls_fields.iter().any(|key| proxy.contains_key(*key)) {
        return Err(EngineServiceError::NodeUnsupported);
    }
    if proxy.contains_key("certificate") != proxy.contains_key("private-key") {
        return Err(EngineServiceError::NodeUnsupported);
    }
    if let Some(value) = proxy.get("fingerprint") {
        let normalized = string_value(value)?.replace(':', "");
        let normalized = normalized.trim();
        if normalized.len() != 64 || !normalized.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(EngineServiceError::NodeUnsupported);
        }
    }
    let user = proxy.get("username").and_then(Value::as_str).unwrap_or("");
    let pass = proxy.get("password").and_then(Value::as_str).unwrap_or("");
    if (protocol == "http" && (user.is_empty() != pass.is_empty()))
        || (protocol == "socks5"
            && (user.len() > 255 || pass.len() > 255 || (user.is_empty() && !pass.is_empty())))
    {
        return Err(EngineServiceError::NodeUnsupported);
    }
    Ok(Value::Object(proxy))
}

fn required_string<'a>(
    input: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, EngineServiceError> {
    input
        .get(key)
        .ok_or(EngineServiceError::NodeUnsupported)
        .and_then(string_value)
}

fn string_value(value: &Value) -> Result<&str, EngineServiceError> {
    value.as_str().ok_or(EngineServiceError::NodeUnsupported)
}

fn validate_headers(value: &Value) -> Result<(), EngineServiceError> {
    let headers = value
        .as_object()
        .ok_or(EngineServiceError::NodeUnsupported)?;
    for (name, value) in headers {
        let header = string_value(value)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            || header.contains(['\r', '\n'])
        {
            return Err(EngineServiceError::NodeUnsupported);
        }
    }
    Ok(())
}
