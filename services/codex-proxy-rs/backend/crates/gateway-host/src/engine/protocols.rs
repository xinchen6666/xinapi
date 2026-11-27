//! 将四类批准节点映射为 Mihomo v1.19.33 的 TCP 出站字段

use std::collections::BTreeSet;

use gateway_admin::model::engines::{EngineFacts, EngineServiceError};
use serde_json::{Map, Value, json};

/// 四协议共享字段按实际出站传输收敛，额外选项返回现有节点诊断
pub(super) fn render_proxy(facts: &EngineFacts) -> Result<Value, EngineServiceError> {
    let input = facts
        .connection
        .as_object()
        .ok_or(EngineServiceError::NodeUnsupported)?;
    let protocol = facts.protocol.as_str();
    if !matches!(protocol, "ss" | "vmess" | "vless" | "trojan")
        || text_field(input, "type")? != protocol
    {
        return Err(EngineServiceError::NodeUnsupported);
    }
    let server = text_field(input, "server")?;
    let port = input
        .get("port")
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .filter(|port| (1..=65535).contains(port))
        .ok_or(EngineServiceError::NodeUnsupported)?;
    let network = match input.get("network") {
        None => "tcp",
        Some(value) => match string(value)? {
            "" | "tcp" => "tcp",
            "ws" if protocol != "ss" => "ws",
            _ => return Err(EngineServiceError::NodeUnsupported),
        },
    };
    let tls = match input.get("tls") {
        Some(value) => value.as_bool().ok_or(EngineServiceError::NodeUnsupported)?,
        None => protocol == "trojan",
    };
    if matches!(protocol, "vless" | "trojan") && !tls {
        return Err(EngineServiceError::NodeUnsupported);
    }
    let mut proxy = Map::from_iter([
        ("name".into(), json!("node")),
        ("type".into(), json!(protocol)),
        ("server".into(), json!(server)),
        ("port".into(), json!(port)),
    ]);
    match protocol {
        "ss" | "trojan" => {
            proxy.insert("password".into(), json!(text_field(input, "password")?));
        }
        "vmess" | "vless" => {
            let uuid = text_field(input, "uuid")?;
            // Admin 规范化 UUID，Host 保持官方协议要求的 16 字节标识形状
            if uuid.len() != 36
                || !uuid.bytes().enumerate().all(|(position, byte)| {
                    if matches!(position, 8 | 13 | 18 | 23) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_hexdigit()
                    }
                })
            {
                return Err(EngineServiceError::NodeUnsupported);
            }
            proxy.insert("uuid".into(), json!(uuid));
        }
        _ => return Err(EngineServiceError::NodeUnsupported),
    }
    if protocol == "ss" {
        let cipher = text_field(input, "cipher")?.to_ascii_lowercase();
        if !matches!(
            cipher.as_str(),
            "aes-128-gcm" | "aes-256-gcm" | "chacha20-ietf-poly1305"
        ) {
            return Err(EngineServiceError::NodeUnsupported);
        }
        proxy.insert("cipher".into(), json!(cipher));
    } else {
        proxy.insert("network".into(), json!(network));
        if protocol != "trojan" {
            proxy.insert("tls".into(), json!(tls));
        }
    }
    if protocol == "vmess" {
        let cipher = input
            .get("cipher")
            .map(string)
            .transpose()?
            .unwrap_or("auto")
            .to_ascii_lowercase();
        if !matches!(
            cipher.as_str(),
            "auto" | "aes-128-gcm" | "chacha20-poly1305" | "none"
        ) || input
            .get("alterId")
            .is_some_and(|value| value.as_u64() != Some(0))
        {
            return Err(EngineServiceError::NodeUnsupported);
        }
        // 订阅层将默认值省略，官方 VMess 客户端需要显式 cipher
        proxy.insert("cipher".into(), json!(cipher));
        proxy.insert("alterId".into(), json!(0));
    }
    for (key, value) in input {
        match key.as_str() {
            "type" | "server" | "port" | "network" => {}
            "password" if matches!(protocol, "ss" | "trojan") => {}
            "uuid" if matches!(protocol, "vmess" | "vless") => {}
            "cipher" if matches!(protocol, "ss" | "vmess") => {}
            "alterId" if protocol == "vmess" => {}
            "tls" if protocol != "ss" => {}
            "udp" | "tfo" | "mptcp" => {
                value.as_bool().ok_or(EngineServiceError::NodeUnsupported)?;
                proxy.insert(key.clone(), value.clone());
            }
            "sni" if tls && protocol == "trojan" => {
                text(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "servername" if tls && matches!(protocol, "vmess" | "vless") => {
                text(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "skip-cert-verify" if tls && protocol != "ss" => {
                value.as_bool().ok_or(EngineServiceError::NodeUnsupported)?;
                proxy.insert(key.clone(), value.clone());
            }
            "name-cert-verify" if tls && protocol != "ss" => {
                text(value)?;
                proxy.insert(key.clone(), value.clone());
            }
            "fingerprint" if tls && protocol != "ss" => {
                let normalized = text(value)?.replace(':', "");
                if normalized.len() != 64
                    || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "client-fingerprint" if tls && protocol != "ss" => {
                if !matches!(
                    string(value)?,
                    "none"
                        | "chrome"
                        | "firefox"
                        | "safari"
                        | "ios"
                        | "android"
                        | "edge"
                        | "360"
                        | "qq"
                        | "random"
                        | "randomized"
                        | "chrome120"
                        | "firefox120"
                        | "safari16"
                ) {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "alpn" if tls && protocol != "ss" => {
                let protocols = value
                    .as_array()
                    .ok_or(EngineServiceError::NodeUnsupported)?;
                for item in protocols {
                    if text(item)?.len() > 255 {
                        return Err(EngineServiceError::NodeUnsupported);
                    }
                }
                // 官方 WS 握手使用 HTTP/1.1，明确拒绝会被底层覆盖的 ALPN
                if network == "ws" && protocols.as_slice() != [json!("http/1.1")] {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "certificate" if tls && protocol != "ss" => {
                if !string(value)?
                    .trim()
                    .starts_with("-----BEGIN CERTIFICATE-----")
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                proxy.insert(key.clone(), value.clone());
            }
            "private-key" if tls && protocol != "ss" => {
                let pem = string(value)?.trim();
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
            "ws-opts" if network == "ws" => {
                proxy.insert(key.clone(), websocket_options(value)?);
            }
            "flow" if protocol == "vless" && value.as_str() == Some("") => {}
            "encryption" if protocol == "vless" && value.as_str() == Some("none") => {}
            _ => return Err(EngineServiceError::NodeUnsupported),
        }
    }
    if proxy.contains_key("certificate") != proxy.contains_key("private-key") {
        return Err(EngineServiceError::NodeUnsupported);
    }
    Ok(Value::Object(proxy))
}

fn text_field<'a>(input: &'a Map<String, Value>, key: &str) -> Result<&'a str, EngineServiceError> {
    text(input.get(key).ok_or(EngineServiceError::NodeUnsupported)?)
}

fn string(value: &Value) -> Result<&str, EngineServiceError> {
    value.as_str().ok_or(EngineServiceError::NodeUnsupported)
}

fn text(value: &Value) -> Result<&str, EngineServiceError> {
    let value = string(value)?;
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(EngineServiceError::NodeUnsupported);
    }
    Ok(value)
}

fn websocket_options(value: &Value) -> Result<Value, EngineServiceError> {
    let input = value
        .as_object()
        .ok_or(EngineServiceError::NodeUnsupported)?;
    let mut options = Map::new();
    for (key, value) in input {
        match key.as_str() {
            "path" => {
                let path = string(value)?;
                if path.chars().any(char::is_control)
                    || (!path.is_empty() && !path.starts_with('/'))
                    || path.starts_with("//")
                    || path.contains('#')
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                let address = reqwest::Url::parse(&format!("https://managed.invalid{path}"))
                    .map_err(|_| EngineServiceError::NodeUnsupported)?;
                // Mihomo 会把 ed 查询参数转换成早发数据，当前传输保持普通握手
                if address
                    .query_pairs()
                    .any(|(name, value)| name == "ed" && !value.is_empty())
                {
                    return Err(EngineServiceError::NodeUnsupported);
                }
                options.insert(key.clone(), value.clone());
            }
            "headers" => {
                let headers = value
                    .as_object()
                    .ok_or(EngineServiceError::NodeUnsupported)?;
                let mut names = BTreeSet::new();
                for (name, value) in headers {
                    http::HeaderName::from_bytes(name.as_bytes())
                        .map_err(|_| EngineServiceError::NodeUnsupported)?;
                    http::HeaderValue::from_str(string(value)?)
                        .map_err(|_| EngineServiceError::NodeUnsupported)?;
                    let canonical = name.to_ascii_lowercase();
                    if matches!(
                        canonical.as_str(),
                        "connection"
                            | "upgrade"
                            | "sec-websocket-key"
                            | "sec-websocket-version"
                            | "sec-websocket-accept"
                            | "sec-websocket-extensions"
                    ) || !names.insert(canonical)
                    {
                        return Err(EngineServiceError::NodeUnsupported);
                    }
                }
                options.insert(key.clone(), value.clone());
            }
            "max-early-data" if value.as_u64() == Some(0) => {}
            "early-data-header-name" if value.as_str() == Some("") => {}
            "v2ray-http-upgrade" | "v2ray-http-upgrade-fast-open"
                if value.as_bool() == Some(false) => {}
            _ => return Err(EngineServiceError::NodeUnsupported),
        }
    }
    Ok(Value::Object(options))
}
