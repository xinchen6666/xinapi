//! 订阅正文的格式判定与逐节点解析
//!
//! 解析只读取节点展示名与协议；脚本、规则、监听、文件路径与 controller 配置一律跳过，
//! 解析器读取调用方已下载的正文，不接触本地文件系统

use base64::Engine as _;
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};
use yaml_rust2::{Yaml, YamlLoader};

use crate::model::subscriptions::{
    DiagnosticCode, MAX_DOCUMENT_BYTES, MAX_NODE_NAME_CHARS, MAX_PROVIDERS, NodeCandidate,
    NodeProtocol, PreviewDiagnostic, SubscriptionFormat,
};

/// 一份文档的解析结果
pub(super) struct ParsedDocument {
    pub(super) format: SubscriptionFormat,
    pub(super) candidates: Vec<NodeCandidate>,
    pub(super) diagnostics: Vec<PreviewDiagnostic>,
    pub(super) providers: Vec<ProviderReference>,
}

/// 待受控拉取的 HTTP 型 provider
pub(super) struct ProviderReference {
    pub(super) name: String,
    pub(super) url: String,
}

/// 文档整体不可读；逐节点问题走诊断，不进入这里
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UnreadableDocument;

/// 解析一份文档；Base64 整体包装解出内层后再按内层格式解析
pub(super) fn parse_document(body: &[u8]) -> Result<ParsedDocument, UnreadableDocument> {
    let text = std::str::from_utf8(body).map_err(|_| UnreadableDocument)?;
    let text = text.trim_start_matches('\u{feff}');
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(UnreadableDocument);
    }
    if let Some(decoded) = decode_wrapped_base64(trimmed) {
        let mut document = parse_plain(&decoded)?;
        // 包装层决定 wire 上的格式，内层格式只影响解析路径
        document.format = SubscriptionFormat::Base64Document;
        return Ok(document);
    }
    parse_plain(trimmed)
}

/// 判定整段正文是否为 Base64 包装：正文全部落在 Base64 字母表内才算包装
fn decode_wrapped_base64(text: &str) -> Option<String> {
    let compact: String = text
        .chars()
        .filter(|value| !value.is_ascii_whitespace())
        .collect();
    if compact.len() < 32 || compact.len() > MAX_DOCUMENT_BYTES {
        return None;
    }
    let looks_wrapped = compact
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '/' | '-' | '_' | '='));
    if !looks_wrapped {
        return None;
    }
    let decoded = String::from_utf8(decode_base64(&compact)?).ok()?;
    if decoded.trim().is_empty() {
        return None;
    }
    Some(decoded)
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

fn parse_plain(text: &str) -> Result<ParsedDocument, UnreadableDocument> {
    let trimmed = text.trim();
    // JSON 先于 YAML：YAML 是 JSON 的超集，先按 JSON 读可保留 clash_json 的格式事实
    if let Some(value) = json_document(trimmed) {
        return document_from_root(&value, SubscriptionFormat::ClashJson);
    }
    if let Some(root) = yaml_clash_root(trimmed) {
        return document_from_clash_root(&root, SubscriptionFormat::ClashYaml);
    }
    parse_uri_list(trimmed)
}

fn json_document(text: &str) -> Option<Value> {
    let looks_like_json = text.starts_with('{') || text.starts_with('[');
    looks_like_json
        .then(|| serde_json::from_str::<Value>(text).ok())
        .flatten()
}

fn yaml_clash_root(text: &str) -> Option<Map<String, Value>> {
    let root = yaml_root(text)?;
    let has_nodes = root.contains_key("proxies") || root.contains_key("proxy-providers");
    has_nodes.then_some(root)
}

fn yaml_root(text: &str) -> Option<Map<String, Value>> {
    let documents = YamlLoader::load_from_str(text).ok()?;
    yaml_to_json(documents.first()?)?.as_object().cloned()
}

/// YAML 值转换为等价 JSON 值：节点读取只保留一条通用路径
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
                // 重复键保留首次出现，与文档作者的书写顺序一致
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
) -> Result<ParsedDocument, UnreadableDocument> {
    let root = value.as_object().ok_or(UnreadableDocument)?;
    if !root.contains_key("proxies") && !root.contains_key("proxy-providers") {
        return Err(UnreadableDocument);
    }
    document_from_clash_root(root, format)
}

fn document_from_clash_root(
    root: &Map<String, Value>,
    format: SubscriptionFormat,
) -> Result<ParsedDocument, UnreadableDocument> {
    let proxies: &[Value] = match root.get("proxies") {
        Some(value) => value.as_array().ok_or(UnreadableDocument)?,
        None => &[],
    };
    // 主文档也可能只带 provider 声明，此时节点全部来自 provider
    let (candidates, mut diagnostics) = collect_nodes(proxies);
    let (providers, provider_diagnostics) = collect_providers(root);
    diagnostics.extend(provider_diagnostics);
    Ok(ParsedDocument {
        format,
        candidates,
        diagnostics,
        providers,
    })
}

fn collect_nodes(proxies: &[Value]) -> (Vec<NodeCandidate>, Vec<PreviewDiagnostic>) {
    let mut candidates = Vec::with_capacity(proxies.len());
    let mut diagnostics = Vec::new();
    for entry in proxies {
        match node_from_entry(entry) {
            Ok(candidate) => {
                if !candidate.supported {
                    diagnostics.push(PreviewDiagnostic::new(
                        DiagnosticCode::UnsupportedProtocol,
                        unsupported_message(candidate.protocol),
                        candidate.name.clone(),
                    ));
                }
                candidates.push(candidate);
            }
            Err((message, subject)) => {
                diagnostics.push(PreviewDiagnostic::new(
                    DiagnosticCode::NodeParseError,
                    message,
                    subject.clone(),
                ));
                // 识别失败的条目同样进入候选，保持 accepted 与 rejected 之和等于候选数量
                candidates.push(NodeCandidate {
                    name: subject,
                    protocol: NodeProtocol::Unknown,
                    supported: false,
                });
            }
        }
    }
    (candidates, diagnostics)
}

fn node_from_entry(entry: &Value) -> Result<NodeCandidate, (&'static str, String)> {
    let Some(object) = entry.as_object() else {
        return Err(("The node entry is not a mapping.", String::new()));
    };
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(sanitize_name)
        .unwrap_or_default();
    if name.is_empty() {
        return Err(("The node entry has no readable name.", "name".to_owned()));
    }
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return Err(("The node entry has no readable type.", name));
    };
    Ok(NodeCandidate::new(name, clash_protocol(kind)))
}

fn collect_providers(
    root: &Map<String, Value>,
) -> (Vec<ProviderReference>, Vec<PreviewDiagnostic>) {
    let mut providers = Vec::new();
    let mut diagnostics = Vec::new();
    let Some(entries) = root.get("proxy-providers").and_then(Value::as_object) else {
        return (providers, diagnostics);
    };
    for (name, entry) in entries {
        let name = sanitize_name(name);
        // 文件型 provider 依赖本地路径，整段跳过
        let is_http = entry
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.eq_ignore_ascii_case("http"));
        if !is_http {
            continue;
        }
        let url = entry
            .get("url")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(url) = url else {
            diagnostics.push(PreviewDiagnostic::new(
                DiagnosticCode::ProviderFetchFailed,
                "The proxy provider entry holds no download URL.",
                name,
            ));
            continue;
        };
        if providers.len() >= MAX_PROVIDERS {
            diagnostics.push(PreviewDiagnostic::new(
                DiagnosticCode::ProviderFetchFailed,
                "The document holds more proxy providers than this gateway downloads.",
                name,
            ));
            continue;
        }
        providers.push(ProviderReference {
            name,
            url: url.to_owned(),
        });
    }
    (providers, diagnostics)
}

fn parse_uri_list(text: &str) -> Result<ParsedDocument, UnreadableDocument> {
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    let mut recognized = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let Some((scheme, rest)) = line
            .split_once("://")
            .filter(|(scheme, _)| valid_scheme(scheme))
        else {
            diagnostics.push(PreviewDiagnostic::new(
                DiagnosticCode::NodeParseError,
                "The line holds no readable node scheme.",
                "",
            ));
            candidates.push(NodeCandidate {
                name: String::new(),
                protocol: NodeProtocol::Unknown,
                supported: false,
            });
            continue;
        };
        recognized = true;
        let candidate = NodeCandidate::new(uri_name(line, scheme, rest), scheme_protocol(scheme));
        if !candidate.supported {
            diagnostics.push(PreviewDiagnostic::new(
                DiagnosticCode::UnsupportedProtocol,
                unsupported_message(candidate.protocol),
                candidate.name.clone(),
            ));
        }
        candidates.push(candidate);
    }
    if !recognized {
        return Err(UnreadableDocument);
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
        && scheme.starts_with(|value: char| value.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
}

fn uri_name(line: &str, scheme: &str, rest: &str) -> String {
    let fragment = line
        .split_once('#')
        .map(|(_, fragment)| {
            percent_decode_str(fragment)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_default();
    let name = sanitize_name(&fragment);
    if !name.is_empty() {
        return name;
    }
    // v2rayN 导出的 vmess 链接把展示名放在 Base64 JSON 的 ps 字段
    if scheme.eq_ignore_ascii_case("vmess") {
        let payload = rest
            .split('#')
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        let name = decode_base64(payload)
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|value| value.get("ps").and_then(Value::as_str).map(sanitize_name))
            .unwrap_or_default();
        if !name.is_empty() {
            return name;
        }
    }
    String::new()
}

fn clash_protocol(kind: &str) -> NodeProtocol {
    match kind.trim().to_ascii_lowercase().as_str() {
        "ss" => NodeProtocol::Ss,
        "vmess" => NodeProtocol::Vmess,
        "vless" => NodeProtocol::Vless,
        "trojan" => NodeProtocol::Trojan,
        "hysteria2" | "hy2" => NodeProtocol::Hysteria2,
        "tuic" => NodeProtocol::Tuic,
        "socks5" => NodeProtocol::Socks5,
        "http" => NodeProtocol::Http,
        _ => NodeProtocol::Unknown,
    }
}

fn scheme_protocol(scheme: &str) -> NodeProtocol {
    match scheme.to_ascii_lowercase().as_str() {
        "ss" => NodeProtocol::Ss,
        "vmess" => NodeProtocol::Vmess,
        "vless" => NodeProtocol::Vless,
        "trojan" => NodeProtocol::Trojan,
        "hysteria2" | "hy2" => NodeProtocol::Hysteria2,
        "tuic" => NodeProtocol::Tuic,
        "socks5" | "socks" => NodeProtocol::Socks5,
        "http" | "https" => NodeProtocol::Http,
        _ => NodeProtocol::Unknown,
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

/// 展示名去掉控制字符与首尾空白，并限制长度
fn sanitize_name(raw: &str) -> String {
    raw.chars()
        .filter(|value| !value.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_NODE_NAME_CHARS)
        .collect()
}
