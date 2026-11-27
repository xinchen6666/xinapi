//! 私有节点连接文档的白名单提取与 URI 到 Clash 连接形状的规范化

use std::{collections::BTreeMap, net::IpAddr};

use base64::Engine as _;
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};
use url::Url;

use crate::model::{subscription_sources::ConnectionDocument, subscriptions::NodeProtocol};

const OPTION_ERROR: &str = "The node connection parameters need correction.";
const VALUE_ERROR: &str = "The node connection parameters require valid field values.";

/// 只提取连接职责字段，额外配置通过节点诊断交由管理员处理
pub(super) fn from_clash(
    entry: &Value,
    protocol: NodeProtocol,
) -> Result<Option<ConnectionDocument>, &'static str> {
    if protocol == NodeProtocol::Unknown {
        return Ok(None);
    }
    let object = entry.as_object().ok_or(VALUE_ERROR)?;
    let mut result = Map::new();
    for (key, value) in object {
        if matches!(key.as_str(), "name" | "icon" | "remark" | "remarks") {
            continue;
        }
        let key = match key.as_str() {
            "sni" | "servername" => sni_key(protocol),
            other => other,
        };
        if !connection_key(key) {
            return Err(OPTION_ERROR);
        }
        validate_nested(key, value)?;
        put(&mut result, key, canonical_value(value))?;
    }
    result.insert("type".into(), Value::from(protocol.as_str()));
    if object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("https"))
    {
        put(&mut result, "tls", Value::Bool(true))?;
    }
    let server = result
        .get("server")
        .and_then(Value::as_str)
        .ok_or(VALUE_ERROR)?;
    result.insert("server".into(), Value::from(canonical_server(server)?));
    let port = result.get("port").ok_or(VALUE_ERROR)?;
    result.insert("port".into(), Value::from(port_number(port)?));
    if let Some(uuid) = result.get("uuid") {
        let uuid =
            uuid::Uuid::parse_str(uuid.as_str().ok_or(VALUE_ERROR)?).map_err(|_| VALUE_ERROR)?;
        result.insert("uuid".into(), Value::from(uuid.to_string()));
    }
    // 认证与 TLS 直接决定出站握手，只接受对应标量类型
    if result
        .get("username")
        .is_some_and(|value| !value.is_string())
        || result
            .get("password")
            .is_some_and(|value| !value.is_string())
        || result.get("tls").is_some_and(|value| !value.is_boolean())
    {
        return Err(VALUE_ERROR);
    }
    // 统一协议默认值，展示改名与等价写法承接同一节点身份
    if result.get("network").and_then(Value::as_str) == Some("tcp") {
        result.remove("network");
    }
    if matches!(
        protocol,
        NodeProtocol::Http | NodeProtocol::Socks5 | NodeProtocol::Vmess | NodeProtocol::Vless
    ) && result.get("tls") == Some(&Value::Bool(false))
    {
        result.remove("tls");
    }
    if matches!(protocol, NodeProtocol::Vmess | NodeProtocol::Vless)
        && result.get("alterId").and_then(Value::as_u64) == Some(0)
    {
        result.remove("alterId");
    }
    // 通用导出器为 VLESS 附带的空 flow 与默认值均归一到省略形状
    if protocol == NodeProtocol::Vless
        && result
            .get("flow")
            .is_some_and(|value| value.is_null() || value.as_str() == Some(""))
    {
        result.remove("flow");
    }
    if matches!(protocol, NodeProtocol::Vmess | NodeProtocol::Vless)
        && result.get("cipher").and_then(Value::as_str) == Some("auto")
    {
        result.remove("cipher");
    }
    Ok(Some(ConnectionDocument(canonical_value(&Value::Object(
        result,
    )))))
}

fn connection_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "server"
            | "port"
            | "cipher"
            | "password"
            | "username"
            | "uuid"
            | "alterId"
            | "udp"
            | "tls"
            | "sni"
            | "servername"
            | "skip-cert-verify"
            | "name-cert-verify"
            | "alpn"
            | "client-fingerprint"
            | "fingerprint"
            | "certificate"
            | "private-key"
            | "ca"
            | "ca-str"
            | "network"
            | "dns-mode"
            | "ws-opts"
            | "http-opts"
            | "h2-opts"
            | "grpc-opts"
            | "reality-opts"
            | "plugin"
            | "plugin-opts"
            | "smux"
            | "flow"
            | "packet-encoding"
            | "packet-addr"
            | "xudp"
            | "global-padding"
            | "authenticated-length"
            | "udp-over-tcp"
            | "udp-over-tcp-version"
            | "tfo"
            | "mptcp"
            | "ip-version"
            | "interface-name"
            | "routing-mark"
            | "dialer-proxy"
            | "obfs"
            | "obfs-password"
            | "up"
            | "down"
            | "ports"
            | "hop-interval"
            | "recv-window"
            | "recv-window-conn"
            | "recv-window-size"
            | "recv-window-conn-size"
            | "max-stream-receive-window"
            | "max-connection-receive-window"
            | "initial-stream-receive-window"
            | "initial-connection-receive-window"
            | "max-idle-timeout"
            | "keep-alive-period"
            | "disable-mtu-discovery"
            | "fast-open"
            | "congestion-controller"
            | "udp-relay-mode"
            | "disable-sni"
            | "reduce-rtt"
            | "heartbeat-interval"
            | "request-timeout"
            | "max-udp-relay-packet-size"
            | "max-open-streams"
            | "ech-opts"
            | "http-headers"
            | "headers"
            | "padding"
            | "encryption"
            | "quic-opts"
    )
}

fn validate_nested(key: &str, value: &Value) -> Result<(), &'static str> {
    let fields: &[&str] = match key {
        "ws-opts" => &[
            "path",
            "headers",
            "max-early-data",
            "early-data-header-name",
            "v2ray-http-upgrade",
            "v2ray-http-upgrade-fast-open",
        ],
        "http-opts" => &["method", "path", "headers"],
        "h2-opts" => &["host", "path"],
        "grpc-opts" => &["grpc-service-name", "grpc-user-agent"],
        "reality-opts" => &["public-key", "short-id", "support-x25519mlkem768"],
        "plugin-opts" => &[
            "mode",
            "host",
            "path",
            "tls",
            "mux",
            "skip-cert-verify",
            "headers",
            "obfs",
            "obfs-host",
            "obfs-uri",
            "version",
            "password",
            "shadow-tls-password",
        ],
        "smux" => &[
            "enabled",
            "protocol",
            "max-connections",
            "min-streams",
            "max-streams",
            "statistic",
            "only-tcp",
            "padding",
            "brutal-opts",
        ],
        "brutal-opts" => &["enabled", "up", "down"],
        "ech-opts" => {
            let object = value.as_object().ok_or(VALUE_ERROR)?;
            for (field, item) in object {
                match field.as_str() {
                    "enable" if item.is_boolean() => {}
                    "config" if item.is_string() => {}
                    "query-server-name"
                        if item.as_str().is_some_and(|name| {
                            !name.is_empty()
                                && !name.chars().any(char::is_whitespace)
                                && !name.chars().any(char::is_control)
                        }) => {}
                    "enable" | "config" | "query-server-name" => return Err(VALUE_ERROR),
                    _ => return Err(OPTION_ERROR),
                }
            }
            return Ok(());
        }
        "quic-opts" => &["key", "security", "header"],
        "headers" | "http-headers" => {
            let object = value.as_object().ok_or(VALUE_ERROR)?;
            if object.values().all(|v| {
                v.is_string() || v.as_array().is_some_and(|a| a.iter().all(Value::is_string))
            }) {
                return Ok(());
            }
            return Err(VALUE_ERROR);
        }
        _ => {
            if value.is_object()
                || value
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v.is_object() || v.is_array()))
            {
                return Err(VALUE_ERROR);
            }
            return Ok(());
        }
    };
    let object = value.as_object().ok_or(VALUE_ERROR)?;
    for (field, item) in object {
        if !fields.contains(&field.as_str()) {
            return Err(OPTION_ERROR);
        }
        validate_nested(field, item)?;
    }
    Ok(())
}

fn canonical_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(k, v)| (k.clone(), canonical_value(v)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(canonical_value).collect()),
        other => other.clone(),
    }
}

fn canonical_server(server: &str) -> Result<String, &'static str> {
    let server = server.trim_matches(['[', ']']);
    if let Ok(ip) = server.parse::<IpAddr>() {
        return Ok(ip.to_string());
    }
    url::Host::parse(server)
        .map(|host| host.to_string().to_ascii_lowercase())
        .map_err(|_| VALUE_ERROR)
}

fn port_number(value: &Value) -> Result<u16, &'static str> {
    let number = value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or(VALUE_ERROR)?;
    u16::try_from(number)
        .ok()
        .filter(|port| *port > 0)
        .ok_or(VALUE_ERROR)
}

fn put(object: &mut Map<String, Value>, key: &str, value: Value) -> Result<(), &'static str> {
    if object.get(key).is_some_and(|existing| *existing != value) {
        return Err(VALUE_ERROR);
    }
    object.insert(key.into(), value);
    Ok(())
}

fn nested(
    object: &mut Map<String, Value>,
    group: &str,
    key: &str,
    value: Value,
) -> Result<(), &'static str> {
    let entry = object
        .entry(group.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    put(entry.as_object_mut().ok_or(VALUE_ERROR)?, key, value)
}

fn sni_key(protocol: NodeProtocol) -> &'static str {
    match protocol {
        NodeProtocol::Vmess | NodeProtocol::Vless => "servername",
        _ => "sni",
    }
}

fn decoded(value: &str) -> Result<String, &'static str> {
    percent_decode_str(value)
        .decode_utf8()
        .map(|v| v.into_owned())
        .map_err(|_| VALUE_ERROR)
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    [STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD]
        .iter()
        .find_map(|engine| engine.decode(value).ok())
}

pub(super) fn from_uri(
    line: &str,
    protocol: NodeProtocol,
) -> Result<Option<ConnectionDocument>, &'static str> {
    if protocol == NodeProtocol::Unknown {
        return Ok(None);
    }
    let (_, rest) = line.split_once("://").ok_or(VALUE_ERROR)?;
    if protocol == NodeProtocol::Vmess {
        return vmess(rest).and_then(|value| from_clash(&Value::Object(value), protocol));
    }
    if protocol == NodeProtocol::Ss {
        return shadowsocks(rest).and_then(|value| from_clash(&Value::Object(value), protocol));
    }
    let url = Url::parse(line).map_err(|_| VALUE_ERROR)?;
    let mut result = endpoint(&url, protocol)?;
    if !matches!(protocol, NodeProtocol::Http | NodeProtocol::Socks5) && url.username().is_empty() {
        return Err(VALUE_ERROR);
    }
    let username = decoded(url.username())?;
    let password = url.password().map(decoded).transpose()?;
    match protocol {
        NodeProtocol::Vless => {
            if password.is_some() {
                return Err(OPTION_ERROR);
            }
            result.insert("uuid".into(), Value::from(username));
        }
        NodeProtocol::Trojan | NodeProtocol::Hysteria2 => {
            let password =
                password.map_or(username.clone(), |suffix| format!("{username}:{suffix}"));
            result.insert("password".into(), Value::from(password));
        }
        NodeProtocol::Tuic => {
            result.insert("uuid".into(), Value::from(username));
            result.insert("password".into(), Value::from(password.ok_or(VALUE_ERROR)?));
        }
        NodeProtocol::Http | NodeProtocol::Socks5 => {
            if !username.is_empty() {
                result.insert("username".into(), Value::from(username));
            }
            if let Some(password) = password {
                result.insert("password".into(), Value::from(password));
            }
            if url.scheme() == "https" {
                result.insert("tls".into(), Value::Bool(true));
            }
            // socks5h 交由代理解析域名，socks5 本地解析；两种模式产生不同的出站行为
            if url.scheme() == "socks5h" {
                result.insert("dns-mode".into(), Value::from("remote"));
            }
        }
        _ => return Err(OPTION_ERROR),
    }
    if !matches!(url.path(), "" | "/") {
        return Err(OPTION_ERROR);
    }
    apply_query(&mut result, &url, protocol)?;
    from_clash(&Value::Object(result), protocol)
}

fn endpoint(url: &Url, protocol: NodeProtocol) -> Result<Map<String, Value>, &'static str> {
    let port = url
        .port_or_known_default()
        .or(match protocol {
            NodeProtocol::Trojan | NodeProtocol::Hysteria2 | NodeProtocol::Tuic => Some(443),
            _ => None,
        })
        .filter(|port| *port > 0)
        .ok_or(VALUE_ERROR)?;
    let mut result = Map::new();
    result.insert("type".into(), Value::from(protocol.as_str()));
    result.insert(
        "server".into(),
        Value::from(url.host_str().ok_or(VALUE_ERROR)?),
    );
    result.insert("port".into(), Value::from(port));
    Ok(result)
}

fn query_map(url: &Url) -> Result<BTreeMap<String, String>, &'static str> {
    let mut result = BTreeMap::new();
    // 严格解码保留认证及传输参数的真实语义
    for pair in url
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|v| !v.is_empty())
    {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decoded(&key.replace('+', " "))?;
        let value = decoded(&value.replace('+', " "))?;
        if result.get(&key).is_some_and(|old| old != &value) {
            return Err(VALUE_ERROR);
        }
        result.insert(key, value);
    }
    Ok(result)
}

fn boolean(value: &str) -> Result<Value, &'static str> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(Value::Bool(true)),
        "0" | "false" => Ok(Value::Bool(false)),
        _ => Err(VALUE_ERROR),
    }
}

fn number(value: &str) -> Result<Value, &'static str> {
    value
        .parse::<u64>()
        .map(Value::from)
        .map_err(|_| VALUE_ERROR)
}

fn string_list(value: &str) -> Value {
    Value::Array(value.split(',').map(|v| Value::from(v.trim())).collect())
}

fn apply_query(
    result: &mut Map<String, Value>,
    url: &Url,
    protocol: NodeProtocol,
) -> Result<(), &'static str> {
    let mut query = query_map(url)?;
    let network = query.remove("type").or_else(|| query.remove("network"));
    if let Some(network) = network.as_deref() {
        if !matches!(protocol, NodeProtocol::Vless | NodeProtocol::Trojan)
            || !matches!(network, "tcp" | "ws" | "grpc" | "h2" | "http")
        {
            return Err(OPTION_ERROR);
        }
        if network != "tcp" {
            put(result, "network", Value::from(network))?;
        }
    }
    let security = query.remove("security");
    if let Some(security) = security.as_deref() {
        match security {
            "tls" | "reality" => put(result, "tls", Value::Bool(true))?,
            "none" | "" => put(result, "tls", Value::Bool(false))?,
            _ => return Err(OPTION_ERROR),
        }
    }
    for (key, value) in query {
        let text = Value::from(value.as_str());
        match key.as_str() {
            "sni" | "peer" | "servername" => put(result, sni_key(protocol), text)?,
            "alpn" => put(result, "alpn", string_list(&value))?,
            "fp" | "client-fingerprint" => put(result, "client-fingerprint", text)?,
            "allowInsecure" | "allow_insecure" | "allow-insecure" | "insecure"
            | "skip-cert-verify" => put(result, "skip-cert-verify", boolean(&value)?)?,
            "tls" | "udp" | "tfo" | "mptcp" | "udp-over-tcp" | "xudp" | "packet-addr" => {
                put(result, &key, boolean(&value)?)?
            }
            "uot" => put(result, "udp-over-tcp", boolean(&value)?)?,
            "udp-over-tcp-version" | "heartbeat-interval" | "hop-interval" => {
                put(result, &key, number(&value)?)?
            }
            "flow"
            | "packet-encoding"
            | "obfs"
            | "obfs-password"
            | "up"
            | "down"
            | "ports"
            | "ip-version"
            | "congestion-controller"
            | "udp-relay-mode" => put(result, &key, text)?,
            "packetEncoding" => put(result, "packet-encoding", text)?,
            "congestion_control" => put(result, "congestion-controller", text)?,
            "udp_relay_mode" => put(result, "udp-relay-mode", text)?,
            "disable_sni" | "disable-sni" => put(result, "disable-sni", boolean(&value)?)?,
            "zero_rtt" | "zero_rtt_handshake" | "reduce-rtt" => {
                put(result, "reduce-rtt", boolean(&value)?)?
            }
            "heartbeat" | "heartbeat_interval" => {
                put(result, "heartbeat-interval", number(&value)?)?
            }
            "mport" => put(result, "ports", text)?,
            "encryption" if protocol == NodeProtocol::Vless && value == "none" => {}
            "pbk" | "public-key" if security.as_deref() == Some("reality") => {
                nested(result, "reality-opts", "public-key", text)?
            }
            "sid" | "short-id" if security.as_deref() == Some("reality") => {
                nested(result, "reality-opts", "short-id", text)?
            }
            "host" => match network.as_deref() {
                Some("ws") => {
                    let group = result
                        .entry("ws-opts".to_owned())
                        .or_insert_with(|| Value::Object(Map::new()))
                        .as_object_mut()
                        .ok_or(VALUE_ERROR)?;
                    nested(group, "headers", "Host", text)?;
                }
                Some("h2") => nested(result, "h2-opts", "host", string_list(&value))?,
                Some("http") => {
                    let group = result
                        .entry("http-opts".to_owned())
                        .or_insert_with(|| Value::Object(Map::new()))
                        .as_object_mut()
                        .ok_or(VALUE_ERROR)?;
                    nested(group, "headers", "Host", Value::Array(vec![text]))?;
                }
                _ => return Err(OPTION_ERROR),
            },
            "path" => match network.as_deref() {
                Some("ws") => nested(result, "ws-opts", "path", text)?,
                Some("h2") => nested(result, "h2-opts", "path", text)?,
                Some("http") => nested(result, "http-opts", "path", Value::Array(vec![text]))?,
                _ => return Err(OPTION_ERROR),
            },
            "serviceName" | "service-name" if network.as_deref() == Some("grpc") => {
                nested(result, "grpc-opts", "grpc-service-name", text)?
            }
            "mode" if network.as_deref() == Some("grpc") && value == "gun" => {}
            "ed" if network.as_deref() == Some("ws") => {
                nested(result, "ws-opts", "max-early-data", number(&value)?)?
            }
            "eh" if network.as_deref() == Some("ws") => {
                nested(result, "ws-opts", "early-data-header-name", text)?
            }
            _ => return Err(OPTION_ERROR),
        }
    }
    if security.as_deref() == Some("reality") && !result.contains_key("reality-opts") {
        return Err(VALUE_ERROR);
    }
    Ok(())
}

fn shadowsocks(rest: &str) -> Result<Map<String, Value>, &'static str> {
    let payload = rest.split('#').next().unwrap_or_default();
    let (encoded, query) = payload.split_once('?').unwrap_or((payload, ""));
    let (credentials, endpoint_text) =
        if let Some((credentials, endpoint)) = encoded.rsplit_once('@') {
            let credentials = decoded(credentials)?;
            let credentials = if credentials.contains(':') {
                credentials
            } else {
                String::from_utf8(decode_base64(&credentials).ok_or(VALUE_ERROR)?)
                    .map_err(|_| VALUE_ERROR)?
            };
            (credentials, endpoint.to_owned())
        } else {
            let full = String::from_utf8(decode_base64(encoded).ok_or(VALUE_ERROR)?)
                .map_err(|_| VALUE_ERROR)?;
            let (credentials, endpoint) = full.rsplit_once('@').ok_or(VALUE_ERROR)?;
            (credentials.to_owned(), endpoint.to_owned())
        };
    let url = Url::parse(&format!("ss://{endpoint_text}?{query}")).map_err(|_| VALUE_ERROR)?;
    if !matches!(url.path(), "" | "/") || !url.username().is_empty() || url.password().is_some() {
        return Err(OPTION_ERROR);
    }
    let mut result = endpoint(&url, NodeProtocol::Ss)?;
    let (cipher, password) = credentials.split_once(':').ok_or(VALUE_ERROR)?;
    result.insert("cipher".into(), Value::from(cipher));
    result.insert("password".into(), Value::from(password));
    let mut query = query_map(&url)?;
    if let Some(plugin) = query.remove("plugin") {
        let mut parts = plugin.split(';');
        let name = match parts.next().ok_or(VALUE_ERROR)? {
            "obfs-local" => "obfs",
            name => name,
        };
        if !matches!(name, "obfs" | "v2ray-plugin" | "shadow-tls") {
            return Err(OPTION_ERROR);
        }
        result.insert("plugin".into(), Value::from(name));
        let mut options = Map::new();
        for part in parts.filter(|v| !v.is_empty()) {
            let (key, value) = part.split_once('=').unwrap_or((part, "true"));
            let key = match key {
                "obfs" => "mode",
                "obfs-host" => "host",
                other => other,
            };
            let value = if matches!(key, "tls" | "mux" | "skip-cert-verify") {
                boolean(value)?
            } else {
                Value::from(value)
            };
            put(&mut options, key, value)?;
        }
        validate_nested("plugin-opts", &Value::Object(options.clone()))?;
        result.insert("plugin-opts".into(), Value::Object(options));
    }
    // 插件参数独立消费，其余查询继续经过公共连接白名单
    let mut common = url;
    common.set_query(None);
    if !query.is_empty() {
        common.query_pairs_mut().extend_pairs(query.iter());
    }
    apply_query(&mut result, &common, NodeProtocol::Ss)?;
    Ok(result)
}

fn vmess(rest: &str) -> Result<Map<String, Value>, &'static str> {
    let payload = rest
        .split('#')
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let value: Value = serde_json::from_slice(&decode_base64(payload).ok_or(VALUE_ERROR)?)
        .map_err(|_| VALUE_ERROR)?;
    let object = value.as_object().ok_or(VALUE_ERROR)?;
    let mut result = Map::new();
    let transport = match object.get("net") {
        Some(value) => value.as_str().ok_or(VALUE_ERROR)?,
        None => "tcp",
    };
    let network =
        if transport == "tcp" && object.get("type").and_then(Value::as_str) == Some("http") {
            "http"
        } else {
            transport
        };
    if !matches!(network, "tcp" | "ws" | "h2" | "http" | "grpc" | "quic") {
        return Err(OPTION_ERROR);
    }
    if network != "tcp" {
        result.insert("network".into(), Value::from(network));
    }
    for (key, value) in object {
        match key.as_str() {
            "ps" | "remark" | "remarks" => {}
            "v" if value.as_str() == Some("2") || value.as_u64() == Some(2) => {}
            "add" => {
                put(&mut result, "server", value.clone())?;
            }
            "port" => {
                put(&mut result, "port", value.clone())?;
            }
            "id" => {
                put(&mut result, "uuid", value.clone())?;
            }
            "aid" => {
                let aid = value
                    .as_u64()
                    .map(Value::from)
                    .or_else(|| value.as_str().and_then(|v| number(v).ok()))
                    .ok_or(VALUE_ERROR)?;
                put(&mut result, "alterId", aid)?;
            }
            "scy" => {
                put(&mut result, "cipher", value.clone())?;
            }
            "net" => {}
            "tls" => {
                let enabled = match value.as_str().ok_or(VALUE_ERROR)? {
                    "tls" => true,
                    "" | "none" => false,
                    _ => return Err(OPTION_ERROR),
                };
                put(&mut result, "tls", Value::Bool(enabled))?;
            }
            "sni" => {
                let name = value.as_str().ok_or(VALUE_ERROR)?;
                if !name.is_empty() {
                    put(&mut result, "servername", value.clone())?;
                }
            }
            "fp" => {
                let name = value.as_str().ok_or(VALUE_ERROR)?;
                if !name.is_empty() {
                    put(&mut result, "client-fingerprint", value.clone())?;
                }
            }
            "alpn" => {
                if let Some(text) = value.as_str().filter(|v| !v.is_empty()) {
                    put(&mut result, "alpn", string_list(text))?;
                } else if value.is_array() {
                    put(&mut result, "alpn", value.clone())?;
                } else if value.as_str() != Some("") {
                    return Err(VALUE_ERROR);
                }
            }
            "allowInsecure" => {
                let flag = if value.is_boolean() {
                    value.clone()
                } else {
                    boolean(value.as_str().ok_or(VALUE_ERROR)?)?
                };
                put(&mut result, "skip-cert-verify", flag)?;
            }
            "type" if value.as_str().is_some_and(|v| matches!(v, "" | "none")) => {}
            "type" if network == "http" && value.as_str() == Some("http") => {}
            "type" if network == "quic" => {
                nested(&mut result, "quic-opts", "header", value.clone())?;
            }
            "host" | "path" if value.as_str() == Some("") => {}
            "host" if network == "ws" => {
                let group = result
                    .entry("ws-opts".to_owned())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .ok_or(VALUE_ERROR)?;
                nested(group, "headers", "Host", value.clone())?;
            }
            "host" if network == "h2" => {
                nested(
                    &mut result,
                    "h2-opts",
                    "host",
                    string_list(value.as_str().ok_or(VALUE_ERROR)?),
                )?;
            }
            "host" if network == "http" => {
                let group = result
                    .entry("http-opts".to_owned())
                    .or_insert_with(|| Value::Object(Map::new()))
                    .as_object_mut()
                    .ok_or(VALUE_ERROR)?;
                nested(
                    group,
                    "headers",
                    "Host",
                    string_list(value.as_str().ok_or(VALUE_ERROR)?),
                )?;
            }
            "path" if matches!(network, "ws" | "h2") => {
                nested(
                    &mut result,
                    if network == "ws" {
                        "ws-opts"
                    } else {
                        "h2-opts"
                    },
                    "path",
                    value.clone(),
                )?;
            }
            "path" if network == "http" => {
                nested(
                    &mut result,
                    "http-opts",
                    "path",
                    Value::Array(vec![value.clone()]),
                )?;
            }
            "path" if network == "grpc" => {
                nested(&mut result, "grpc-opts", "grpc-service-name", value.clone())?;
            }
            "host" if network == "quic" => {
                nested(&mut result, "quic-opts", "security", value.clone())?;
            }
            "path" if network == "quic" => {
                nested(&mut result, "quic-opts", "key", value.clone())?;
            }
            _ => return Err(OPTION_ERROR),
        }
    }
    Ok(result)
}
