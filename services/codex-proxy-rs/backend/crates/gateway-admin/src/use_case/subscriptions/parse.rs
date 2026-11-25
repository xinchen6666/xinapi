//! 订阅正文的格式识别、节点连接参数校验与 provider 声明读取
//!
//! 私有连接文档只保存连接白名单，公开预览沿用展示事实与脱敏诊断

use std::net::IpAddr;

use base64::Engine as _;
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};
use url::Url;
use yaml_rust2::{Yaml, YamlLoader};

use crate::model::{
    subscription_sources::ConnectionDocument,
    subscriptions::{
        DiagnosticCode, MAX_DECOMPRESSED_BYTES, MAX_NODE_NAME_CHARS, MAX_PREVIEW_CANDIDATES,
        MAX_PREVIEW_DIAGNOSTICS, MAX_PROVIDERS, NodeCandidate, NodeProtocol, PreviewDiagnostic,
        SubscriptionFormat, SubscriptionPreviewError,
    },
};

use super::connection;

/// 单份文档的节点、诊断和待下载来源
pub(super) struct ParsedDocument {
    pub(super) format: SubscriptionFormat,
    pub(super) candidates: Vec<NodeCandidate>,
    pub(super) diagnostics: Vec<PreviewDiagnostic>,
    pub(super) providers: Vec<ProviderReference>,
}

/// 由下载边界统一处理的 HTTP provider
pub(super) struct ProviderReference {
    pub(super) name: String,
    pub(super) url: String,
}

pub(super) fn parse_document(body: &[u8]) -> Result<ParsedDocument, SubscriptionPreviewError> {
    if body.len() > MAX_DECOMPRESSED_BYTES {
        return Err(SubscriptionPreviewError::TooLarge);
    }
    let text = std::str::from_utf8(body).map_err(|_| SubscriptionPreviewError::Unreadable)?;
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        return Err(SubscriptionPreviewError::Unreadable);
    }
    let compact: String = text
        .chars()
        .filter(|value| !value.is_ascii_whitespace())
        .collect();
    if compact
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '/' | '-' | '_' | '='))
    {
        let decoded = decode_base64(&compact).ok_or(SubscriptionPreviewError::Unreadable)?;
        let decoded =
            std::str::from_utf8(&decoded).map_err(|_| SubscriptionPreviewError::Unreadable)?;
        let mut parsed = parse_plain(decoded.trim_start_matches('\u{feff}').trim())?;
        parsed.format = SubscriptionFormat::Base64Document;
        return Ok(parsed);
    }
    parse_plain(text)
}

fn decode_base64(value: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
    STANDARD
        .decode(value)
        .ok()
        .or_else(|| STANDARD_NO_PAD.decode(value).ok())
        .or_else(|| URL_SAFE.decode(value).ok())
        .or_else(|| URL_SAFE_NO_PAD.decode(value).ok())
}

fn parse_plain(text: &str) -> Result<ParsedDocument, SubscriptionPreviewError> {
    if text.starts_with('{') || text.starts_with('[') {
        if let Ok(root) = serde_json::from_str::<Value>(text) {
            return document_from_root(&root, SubscriptionFormat::ClashJson);
        }
        return parse_yaml_document(text);
    }
    // 结构化声明先确定格式，URI 列表的逐行错误仍由列表解析器保留
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("//"))
        .collect::<Vec<_>>();
    let structured = lines.iter().any(|line| {
        line.split_once(':').is_some_and(|(key, _)| {
            matches!(
                key.trim().trim_matches(['\"', '\'']),
                "proxies" | "proxy-providers"
            )
        })
    });
    if !structured
        && lines.iter().any(|line| {
            line.split_once("://")
                .is_some_and(|(scheme, _)| valid_scheme(scheme))
        })
    {
        return parse_uri_list(text);
    }
    parse_yaml_document(text)
}

fn parse_yaml_document(text: &str) -> Result<ParsedDocument, SubscriptionPreviewError> {
    let documents =
        YamlLoader::load_from_str(text).map_err(|_| SubscriptionPreviewError::Unreadable)?;
    if documents.len() != 1 {
        return Err(SubscriptionPreviewError::Unreadable);
    }
    let root = documents
        .first()
        .and_then(yaml_to_json)
        .ok_or(SubscriptionPreviewError::Unreadable)?;
    document_from_root(&root, SubscriptionFormat::ClashYaml)
}

fn yaml_to_json(value: &Yaml) -> Option<Value> {
    match value {
        Yaml::Real(raw) => Some(
            raw.parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map_or_else(|| Value::String(raw.clone()), Value::Number),
        ),
        Yaml::Integer(number) => Some(Value::from(*number)),
        Yaml::String(text) => Some(Value::String(text.clone())),
        Yaml::Boolean(flag) => Some(Value::from(*flag)),
        Yaml::Array(items) => Some(Value::Array(
            items.iter().map(yaml_to_json).collect::<Option<Vec<_>>>()?,
        )),
        Yaml::Hash(entries) => {
            let mut object = Map::with_capacity(entries.len());
            for (key, value) in entries {
                let key = match key {
                    Yaml::String(text) => text.clone(),
                    Yaml::Integer(number) => number.to_string(),
                    Yaml::Boolean(flag) => flag.to_string(),
                    _ => continue,
                };
                object
                    .entry(key)
                    .or_insert_with(|| yaml_to_json(value).unwrap_or(Value::Null));
            }
            Some(Value::Object(object))
        }
        Yaml::Alias(_) | Yaml::Null | Yaml::BadValue => Some(Value::Null),
    }
}

fn document_from_root(
    value: &Value,
    format: SubscriptionFormat,
) -> Result<ParsedDocument, SubscriptionPreviewError> {
    let root = value
        .as_object()
        .ok_or(SubscriptionPreviewError::Unreadable)?;
    if !root.contains_key("proxies") && !root.contains_key("proxy-providers") {
        return Err(SubscriptionPreviewError::Unreadable);
    }
    let proxies: &[Value] = match root.get("proxies") {
        Some(value) => value
            .as_array()
            .ok_or(SubscriptionPreviewError::Unreadable)?,
        None => &[],
    };
    if proxies.len() > MAX_PREVIEW_CANDIDATES {
        return Err(SubscriptionPreviewError::PreviewTooLarge);
    }
    let mut candidates = Vec::with_capacity(proxies.len());
    let mut diagnostics = Vec::new();
    for entry in proxies {
        let mut name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(sanitize_name)
            .unwrap_or_default();
        if name.is_empty() && entry.is_object() {
            name = "name".to_owned();
        }
        let protocol = entry
            .get("type")
            .and_then(Value::as_str)
            .map_or(NodeProtocol::Unknown, clash_protocol);
        let issue = validate_clash_node(entry, protocol);
        let connection = if issue.is_none() {
            connection::from_clash(entry, protocol)
        } else {
            Ok(None)
        };
        let issue = issue.or(connection.as_ref().err().copied());
        push_node(
            &mut candidates,
            &mut diagnostics,
            name,
            protocol,
            issue,
            connection.ok().flatten(),
        )?;
    }
    let mut providers = Vec::new();
    if let Some(entries) = root.get("proxy-providers") {
        if let Some(entries) = entries.as_object() {
            for (name, entry) in entries {
                let name = sanitize_name(name);
                let kind = entry.get("type").and_then(Value::as_str);
                let diagnostic = match kind {
                    Some(kind) if kind.eq_ignore_ascii_case("http") => {
                        let url = entry
                            .get("url")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|v| !v.is_empty());
                        match url {
                            Some(url) if providers.len() < MAX_PROVIDERS => {
                                providers.push(ProviderReference {
                                    name,
                                    url: url.to_owned(),
                                });
                                continue;
                            }
                            Some(_) => PreviewDiagnostic::new(
                                DiagnosticCode::ProviderFetchFailed,
                                "The document holds more proxy providers than this gateway downloads.",
                                name,
                            ),
                            None => PreviewDiagnostic::new(
                                DiagnosticCode::ProviderParseError,
                                "The HTTP proxy provider requires a download URL.",
                                name,
                            ),
                        }
                    }
                    Some(_) => PreviewDiagnostic::new(
                        DiagnosticCode::ProviderUnsupported,
                        "Only HTTP proxy providers are expanded for preview.",
                        name,
                    ),
                    None => PreviewDiagnostic::new(
                        DiagnosticCode::ProviderParseError,
                        "The proxy provider requires a mapping with a readable type.",
                        name,
                    ),
                };
                diagnostics.push(diagnostic);
                check_counts(candidates.len(), diagnostics.len())?;
            }
        } else {
            diagnostics.push(PreviewDiagnostic::new(
                DiagnosticCode::ProviderParseError,
                "The proxy-providers field requires a mapping.",
                "proxy-providers",
            ));
        }
    }
    check_counts(candidates.len(), diagnostics.len())?;
    Ok(ParsedDocument {
        format,
        candidates,
        diagnostics,
        providers,
    })
}

/// 数量预算同时约束单份正文和整次 provider 合并
pub(super) fn check_counts(
    candidates: usize,
    diagnostics: usize,
) -> Result<(), SubscriptionPreviewError> {
    if candidates > MAX_PREVIEW_CANDIDATES || diagnostics > MAX_PREVIEW_DIAGNOSTICS {
        return Err(SubscriptionPreviewError::PreviewTooLarge);
    }
    Ok(())
}

fn push_node(
    candidates: &mut Vec<NodeCandidate>,
    diagnostics: &mut Vec<PreviewDiagnostic>,
    name: String,
    protocol: NodeProtocol,
    issue: Option<&'static str>,
    connection: Option<ConnectionDocument>,
) -> Result<(), SubscriptionPreviewError> {
    let mut candidate = NodeCandidate::new(name, protocol);
    candidate.connection = connection;
    if let Some(message) = issue {
        candidate.parse_valid = false;
        candidate.supported = false;
        diagnostics.push(PreviewDiagnostic::new(
            DiagnosticCode::NodeParseError,
            message,
            candidate.name.clone(),
        ));
    } else if !candidate.supported {
        diagnostics.push(PreviewDiagnostic::new(
            DiagnosticCode::UnsupportedProtocol,
            unsupported_message(protocol),
            candidate.name.clone(),
        ));
    }
    candidates.push(candidate);
    check_counts(candidates.len(), diagnostics.len())
}

fn validate_clash_node(entry: &Value, protocol: NodeProtocol) -> Option<&'static str> {
    let Some(object) = entry.as_object() else {
        return Some("The node entry requires a mapping.");
    };
    if object
        .get("name")
        .and_then(Value::as_str)
        .map(sanitize_name)
        .is_none_or(|v| v.is_empty())
    {
        return Some("The node entry requires a readable name.");
    }
    if object
        .get("type")
        .and_then(Value::as_str)
        .is_none_or(|v| v.trim().is_empty())
    {
        return Some("The node entry requires a readable type.");
    }
    // 额外协议只保留识别诊断，连接有效性由已实现的协议校验决定
    if protocol == NodeProtocol::Unknown {
        return None;
    }
    if object
        .get("server")
        .and_then(Value::as_str)
        .is_none_or(|v| !valid_server(v))
    {
        return Some("The node requires a valid server address.");
    }
    if object.get("port").and_then(node_port).is_none() {
        return Some("The node requires a port from 1 to 65535.");
    }
    match protocol {
        NodeProtocol::Ss if !text_field(object, "cipher") || !text_field(object, "password") => {
            Some("The Shadowsocks node requires a cipher and password.")
        }
        NodeProtocol::Vmess | NodeProtocol::Vless if !uuid_field(object, "uuid") => {
            Some("The node requires a valid UUID.")
        }
        NodeProtocol::Trojan | NodeProtocol::Hysteria2 if !text_field(object, "password") => {
            Some("The node requires a password.")
        }
        NodeProtocol::Tuic if !uuid_field(object, "uuid") || !text_field(object, "password") => {
            Some("The TUIC node requires a UUID and password.")
        }
        _ => None,
    }
}

fn text_field(object: &Map<String, Value>, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|v| !v.is_empty() && !v.chars().any(char::is_control))
}
fn uuid_field(object: &Map<String, Value>, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|v| uuid::Uuid::parse_str(v).is_ok())
}
fn node_port(value: &Value) -> Option<u16> {
    let port = value
        .as_u64()
        .and_then(|v| u16::try_from(v).ok())
        .or_else(|| value.as_str()?.parse::<u16>().ok())?;
    (port > 0).then_some(port)
}
fn valid_server(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(|v| v.is_whitespace() || v.is_control())
        && (value.parse::<IpAddr>().is_ok() || url::Host::parse(value).is_ok())
}

fn parse_uri_list(text: &str) -> Result<ParsedDocument, SubscriptionPreviewError> {
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    let mut recognized = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some((scheme, rest)) = line.split_once("://").filter(|(s, _)| valid_scheme(s)) else {
            push_node(
                &mut candidates,
                &mut diagnostics,
                String::new(),
                NodeProtocol::Unknown,
                Some("The line requires a readable node scheme."),
                None,
            )?;
            continue;
        };
        recognized = true;
        let protocol = scheme_protocol(scheme);
        let name = uri_name(line, scheme, rest);
        let issue = validate_uri(line, rest, protocol);
        let connection = if issue.is_none() {
            connection::from_uri(line, protocol)
        } else {
            Ok(None)
        };
        let issue = issue.or(connection.as_ref().err().copied());
        push_node(
            &mut candidates,
            &mut diagnostics,
            name,
            protocol,
            issue,
            connection.ok().flatten(),
        )?;
    }
    if !recognized {
        return Err(SubscriptionPreviewError::Unreadable);
    }
    Ok(ParsedDocument {
        format: SubscriptionFormat::UriList,
        candidates,
        diagnostics,
        providers: Vec::new(),
    })
}

fn valid_scheme(scheme: &str) -> bool {
    (1..=24).contains(&scheme.len())
        && scheme.starts_with(|v: char| v.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|v| v.is_ascii_alphanumeric() || matches!(v, '+' | '-' | '.'))
}

fn validate_uri(line: &str, rest: &str, protocol: NodeProtocol) -> Option<&'static str> {
    if protocol == NodeProtocol::Unknown {
        return None;
    }
    if protocol == NodeProtocol::Vmess {
        let payload = rest
            .split('#')
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        let Some(value) =
            decode_base64(payload).and_then(|v| serde_json::from_slice::<Value>(&v).ok())
        else {
            return Some("The VMess URI requires a Base64 JSON payload.");
        };
        let Some(object) = value.as_object() else {
            return Some("The VMess payload requires a mapping.");
        };
        if object
            .get("add")
            .and_then(Value::as_str)
            .is_none_or(|v| !valid_server(v))
            || object.get("port").and_then(node_port).is_none()
            || !uuid_field(object, "id")
        {
            return Some("The VMess payload requires a server, port and valid UUID.");
        }
        return None;
    }
    if protocol == NodeProtocol::Ss {
        let payload = rest.split('#').next().unwrap_or_default();
        let encoded = payload.split('?').next().unwrap_or_default();
        let parsed = if let Some((userinfo, endpoint)) = encoded.rsplit_once('@') {
            let Ok(decoded) = percent_decode_str(userinfo).decode_utf8() else {
                return Some("The Shadowsocks URI requires UTF-8 credentials.");
            };
            let credentials = if decoded.contains(':') {
                decoded.into_owned()
            } else {
                decode_base64(&decoded)
                    .and_then(|v| String::from_utf8(v).ok())
                    .unwrap_or_default()
            };
            (credentials, endpoint.to_owned())
        } else {
            let Some(decoded) = decode_base64(encoded).and_then(|v| String::from_utf8(v).ok())
            else {
                return Some("The Shadowsocks URI requires encoded credentials and a server.");
            };
            let Some((credentials, endpoint)) = decoded.rsplit_once('@') else {
                return Some("The Shadowsocks URI requires credentials and a server.");
            };
            (credentials.to_owned(), endpoint.to_owned())
        };
        let credentials_valid = parsed
            .0
            .split_once(':')
            .is_some_and(|(cipher, password)| !cipher.is_empty() && !password.is_empty());
        let endpoint_valid = Url::parse(&format!("ss://{}", parsed.1))
            .ok()
            .is_some_and(|u| u.host_str().is_some() && u.port().is_some_and(|p| p > 0));
        return (!credentials_valid || !endpoint_valid)
            .then_some("The Shadowsocks URI requires a cipher, password, server and port.");
    }
    let Ok(url) = Url::parse(line) else {
        return Some("The node URI has an invalid address or port.");
    };
    if url.host_str().is_none_or(|v| !valid_server(v)) {
        return Some("The node URI requires a server address.");
    }
    let port = url.port_or_known_default().or(match protocol {
        NodeProtocol::Trojan | NodeProtocol::Hysteria2 | NodeProtocol::Tuic => Some(443),
        _ => None,
    });
    if port.is_none_or(|v| v == 0) {
        return Some("The node URI requires a port from 1 to 65535.");
    }
    let user = percent_decode_str(url.username()).decode_utf8().ok();
    match protocol {
        NodeProtocol::Vless
            if user
                .as_ref()
                .is_none_or(|v| uuid::Uuid::parse_str(v).is_err()) =>
        {
            Some("The VLESS URI requires a valid UUID.")
        }
        NodeProtocol::Trojan | NodeProtocol::Hysteria2
            if user.as_ref().is_none_or(|v| v.is_empty()) =>
        {
            Some("The node URI requires a password.")
        }
        NodeProtocol::Tuic
            if user
                .as_ref()
                .is_none_or(|v| uuid::Uuid::parse_str(v).is_err())
                || url.password().is_none_or(str::is_empty) =>
        {
            Some("The TUIC URI requires a UUID and password.")
        }
        _ => None,
    }
}

fn uri_name(line: &str, scheme: &str, rest: &str) -> String {
    let name = line
        .split_once('#')
        .and_then(|(_, v)| percent_decode_str(v).decode_utf8().ok())
        .map(|v| sanitize_name(&v))
        .unwrap_or_default();
    if !name.is_empty() {
        return name;
    }
    if scheme.eq_ignore_ascii_case("vmess") {
        return decode_base64(
            rest.split('#')
                .next()
                .unwrap_or_default()
                .trim_end_matches('/'),
        )
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| v.get("ps").and_then(Value::as_str).map(sanitize_name))
        .unwrap_or_default();
    }
    String::new()
}
fn clash_protocol(kind: &str) -> NodeProtocol {
    match kind.trim().to_ascii_lowercase().as_str() {
        "ss" | "shadowsocks" => NodeProtocol::Ss,
        "vmess" => NodeProtocol::Vmess,
        "vless" => NodeProtocol::Vless,
        "trojan" => NodeProtocol::Trojan,
        "hysteria2" | "hy2" => NodeProtocol::Hysteria2,
        "tuic" => NodeProtocol::Tuic,
        "socks" | "socks5" | "socks5h" => NodeProtocol::Socks5,
        "http" | "https" => NodeProtocol::Http,
        _ => NodeProtocol::Unknown,
    }
}
fn scheme_protocol(scheme: &str) -> NodeProtocol {
    match scheme.to_ascii_lowercase().as_str() {
        "socks" | "socks5h" => NodeProtocol::Socks5,
        "https" => NodeProtocol::Http,
        other => clash_protocol(other),
    }
}
fn unsupported_message(protocol: NodeProtocol) -> String {
    match protocol {
        NodeProtocol::Unknown => {
            "The node protocol is unrecognized; this gateway materializes http and socks5 nodes."
                .to_owned()
        }
        other => format!(
            "Protocol {} is recognized; this gateway materializes http and socks5 nodes.",
            other.as_str()
        ),
    }
}
fn sanitize_name(raw: &str) -> String {
    raw.chars()
        .filter(|v| !v.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_NODE_NAME_CHARS)
        .collect()
}
