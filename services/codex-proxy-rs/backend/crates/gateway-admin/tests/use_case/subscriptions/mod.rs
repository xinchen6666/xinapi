//! 验证订阅预览的格式判定、逐节点结果、provider 物化与脱敏错误分类

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use base64::Engine as _;
use gateway_admin::{
    model::subscriptions::{
        FETCH_BUDGET, NodeProtocol, SubscriptionFetchPolicy, SubscriptionFormat,
        SubscriptionPreview, SubscriptionPreviewError, SubscriptionSourceError,
    },
    ports::subscription::{FetchedDocument, SubscriptionDocumentSource, SubscriptionFetchError},
};

use super::AdminHarness;

const MAIN_URL: &str = "https://sub.example.com/main.yaml";
const PROVIDER_URL: &str = "https://sub.example.com/provider.yaml";
const NESTED_URL: &str = "https://sub.example.com/nested.yaml";

#[derive(Default)]
struct FixtureSource {
    documents: Mutex<BTreeMap<String, Result<Vec<u8>, SubscriptionFetchError>>>,
    budgets: Mutex<Vec<Duration>>,
}

impl FixtureSource {
    fn document(&self, url: &str, body: impl Into<Vec<u8>>) {
        self.documents
            .lock()
            .expect("fixture documents")
            .insert(url.to_owned(), Ok(body.into()));
    }

    fn failure(&self, url: &str, error: SubscriptionFetchError) {
        self.documents
            .lock()
            .expect("fixture documents")
            .insert(url.to_owned(), Err(error));
    }

    fn budgets(&self) -> Vec<Duration> {
        self.budgets.lock().expect("fixture budgets").clone()
    }
}

fn fixtures() -> Arc<FixtureSource> {
    Arc::new(FixtureSource::default())
}

#[async_trait]
impl SubscriptionDocumentSource for FixtureSource {
    async fn fetch(
        &self,
        url: &str,
        _: &SubscriptionFetchPolicy,
        budget: Duration,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        self.budgets.lock().expect("fixture budgets").push(budget);
        match self.documents.lock().expect("fixture documents").get(url) {
            Some(Ok(body)) => Ok(FetchedDocument { body: body.clone() }),
            Some(Err(error)) => Err(*error),
            None => Err(SubscriptionFetchError::Failed),
        }
    }
}

async fn preview(source: Arc<FixtureSource>, url: &str) -> SubscriptionPreview {
    AdminHarness::new()
        .subscriptions(source)
        .build()
        .await
        .subscriptions()
        .preview(url)
        .await
        .expect("subscription preview")
}

async fn preview_error(source: Arc<FixtureSource>, url: &str) -> SubscriptionPreviewError {
    AdminHarness::new()
        .subscriptions(source)
        .build()
        .await
        .subscriptions()
        .preview(url)
        .await
        .expect_err("subscription preview error")
}

fn protocol(preview: &SubscriptionPreview, name: &str) -> NodeProtocol {
    preview
        .candidates
        .iter()
        .find(|candidate| candidate.name == name)
        .unwrap_or_else(|| panic!("candidate {name}"))
        .protocol
}

fn supported(preview: &SubscriptionPreview, name: &str) -> bool {
    preview
        .candidates
        .iter()
        .find(|candidate| candidate.name == name)
        .unwrap_or_else(|| panic!("candidate {name}"))
        .supported
}

#[tokio::test]
async fn clash_yaml_preview_reports_each_node_and_keeps_unmaterialized_protocols() {
    let source = fixtures();
    source.document(
        MAIN_URL,
        r#"
port: 7890
socks-port: 7891
allow-lan: true
mode: rule
proxies:
  - {name: alpha, type: ss, server: 198.51.100.1, port: 8388, cipher: aes-128-gcm, password: secret}
  - {name: beta, type: vmess, server: 198.51.100.2, port: 443, uuid: 11111111-1111-1111-1111-111111111111}
  - {name: gamma, type: socks5, server: 198.51.100.3, port: 1080}
  - {name: delta, type: http, server: 198.51.100.4, port: 8080}
  - {name: epsilon, type: wireguard, server: 198.51.100.5, port: 51820}
  - {type: trojan, server: 198.51.100.6, port: 8443, password: secret}
proxy-groups:
  - {name: manual, type: select, proxies: [alpha, gamma]}
rules:
  - MATCH,manual
"#,
    );
    let result = preview(source, MAIN_URL).await;

    assert_eq!(result.format, SubscriptionFormat::ClashYaml);
    assert_eq!(result.source_url, MAIN_URL);
    assert_eq!(result.candidates.len(), 6);
    assert_eq!(result.accepted, 2);
    assert_eq!(result.rejected, 4);
    assert_eq!(protocol(&result, "alpha"), NodeProtocol::Ss);
    assert_eq!(protocol(&result, "beta"), NodeProtocol::Vmess);
    assert_eq!(protocol(&result, "epsilon"), NodeProtocol::Unknown);
    assert!(supported(&result, "gamma"));
    assert!(supported(&result, "delta"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() == "unsupported_protocol")
            .count(),
        3
    );
    let unreadable = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "node_parse_error")
        .expect("node parse diagnostic");
    assert_eq!(unreadable.subject, "name");
}

#[tokio::test]
async fn clash_json_preview_recognizes_the_same_node_shape() {
    let source = fixtures();
    source.document(
        MAIN_URL,
        r#"{"proxies":[
            {"name":"alpha","type":"trojan","server":"198.51.100.7","port":443,"password":"secret"},
            {"name":"beta","type":"socks5","server":"198.51.100.8","port":1080}
        ]}"#,
    );
    let result = preview(source, MAIN_URL).await;

    assert_eq!(result.format, SubscriptionFormat::ClashJson);
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(result.accepted, 1);
    assert_eq!(result.rejected, 1);
    assert_eq!(protocol(&result, "alpha"), NodeProtocol::Trojan);
    assert!(supported(&result, "beta"));
}

#[tokio::test]
async fn uri_list_preview_reads_fragments_and_reports_unreadable_lines() {
    let source = fixtures();
    source.document(
        MAIN_URL,
        concat!(
            "# exported fixture\n",
            "vmess://eyJ2IjogIjIiLCAicHMiOiAiYWxwaGEiLCAiYWRkIjogIjE5OC41MS4xMDAuOSIsICJwb3J0IjogIjQ0MyIsICJpZCI6ICIxMTExMTExMS0xMTExLTExMTEtMTExMS0xMTExMTExMTExMTEifQ==\n",
            "vless://11111111-1111-1111-1111-111111111111@198.51.100.10:443?security=tls#beta\n",
            "ss://YWVzLTEyOC1nY206c2VjcmV0@198.51.100.11:8388#gamma%20node\n",
            "trojan://secret@198.51.100.12:8443#delta\n",
            "hysteria2://secret@198.51.100.13:443#epsilon\n",
            "tuic://11111111-1111-1111-1111-111111111111:secret@198.51.100.14:443#zeta\n",
            "socks5://198.51.100.15:1080#eta\n",
            "http://198.51.100.16:8080#theta\n",
            "ssr://bm90LWEtbm9kZQ\n",
            "not-a-node-line\n",
        ),
    );
    let result = preview(source, MAIN_URL).await;

    assert_eq!(result.format, SubscriptionFormat::UriList);
    assert_eq!(protocol(&result, "alpha"), NodeProtocol::Vmess);
    assert_eq!(protocol(&result, "beta"), NodeProtocol::Vless);
    assert_eq!(protocol(&result, "gamma node"), NodeProtocol::Ss);
    assert_eq!(protocol(&result, "delta"), NodeProtocol::Trojan);
    assert_eq!(protocol(&result, "epsilon"), NodeProtocol::Hysteria2);
    assert_eq!(protocol(&result, "zeta"), NodeProtocol::Tuic);
    assert!(supported(&result, "eta"));
    assert!(supported(&result, "theta"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code.as_str() == "node_parse_error")
            .count(),
        1
    );
    assert_eq!(result.accepted, 2);
    assert_eq!(result.rejected, result.candidates.len() - 2);
}

#[tokio::test]
async fn base64_wrapped_document_keeps_the_wrapper_format() {
    let inner = "proxies:\n  - {name: alpha, type: socks5, server: 198.51.100.17, port: 1080}\n";
    let wrapped = base64::engine::general_purpose::STANDARD.encode(inner);
    let source = fixtures();
    source.document(MAIN_URL, wrapped);
    let result = preview(source, MAIN_URL).await;

    assert_eq!(result.format, SubscriptionFormat::Base64Document);
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.accepted, 1);
    assert_eq!(protocol(&result, "alpha"), NodeProtocol::Socks5);
}

#[tokio::test]
async fn provider_documents_merge_into_candidates_and_keep_depth_one() {
    let main = format!(
        concat!(
            "proxies:\n",
            "  - {{name: alpha, type: socks5, server: 198.51.100.18, port: 1080}}\n",
            "proxy-providers:\n",
            "  remote:\n",
            "    type: http\n",
            "    url: {provider}\n",
            "    path: ./providers/remote.yaml\n",
            "    interval: 3600\n",
            "  local:\n",
            "    type: file\n",
            "    path: ./providers/local.yaml\n",
            "rules:\n  - MATCH,DIRECT\n"
        ),
        provider = PROVIDER_URL,
    );
    let provider = format!(
        concat!(
            "proxies:\n",
            "  - {{name: beta, type: http, server: 198.51.100.19, port: 8080}}\n",
            "proxy-providers:\n",
            "  nested:\n",
            "    type: http\n",
            "    url: {nested}\n"
        ),
        nested = NESTED_URL,
    );
    let source = fixtures();
    source.document(MAIN_URL, main);
    source.document(PROVIDER_URL, provider);
    source.document(
        NESTED_URL,
        "proxies:\n  - {name: nested, type: socks5, server: 198.51.100.20, port: 1080}\n",
    );
    let result = preview(source, MAIN_URL).await;

    assert!(result.candidates.iter().any(|node| node.name == "alpha"));
    assert!(result.candidates.iter().any(|node| node.name == "beta"));
    assert!(result.candidates.iter().all(|node| node.name != "nested"));
    assert_eq!(result.accepted, 2);
    assert_eq!(result.rejected, 0);
    let depth = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "provider_depth_limit")
        .expect("provider depth diagnostic");
    assert_eq!(depth.subject, "nested");
}

#[tokio::test]
async fn provider_failure_keeps_main_document_nodes_and_reports_the_provider() {
    let main = format!(
        concat!(
            "proxies:\n",
            "  - {{name: alpha, type: socks5, server: 198.51.100.21, port: 1080}}\n",
            "proxy-providers:\n",
            "  remote:\n",
            "    type: http\n",
            "    url: {provider}\n"
        ),
        provider = PROVIDER_URL,
    );
    let source = fixtures();
    source.document(MAIN_URL, main);
    source.failure(PROVIDER_URL, SubscriptionFetchError::Failed);
    let result = preview(source, MAIN_URL).await;

    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.accepted, 1);
    let failure = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "provider_fetch_failed")
        .expect("provider failure diagnostic");
    assert_eq!(failure.subject, "remote");
}

#[tokio::test]
async fn provider_budget_shares_the_single_preview_deadline() {
    let main = format!(
        concat!(
            "proxies:\n",
            "  - {{name: alpha, type: socks5, server: 198.51.100.22, port: 1080}}\n",
            "proxy-providers:\n",
            "  remote:\n",
            "    type: http\n",
            "    url: {provider}\n"
        ),
        provider = PROVIDER_URL,
    );
    let source = fixtures();
    source.document(MAIN_URL, main);
    source.document(
        PROVIDER_URL,
        "proxies:\n  - {name: beta, type: socks5, server: 198.51.100.23, port: 1080}\n",
    );
    let result = preview(Arc::clone(&source), MAIN_URL).await;

    assert_eq!(result.accepted, 2);
    let budgets = source.budgets();
    assert_eq!(budgets.len(), 2);
    assert!(budgets.iter().all(|budget| *budget <= FETCH_BUDGET));
    assert!(budgets[1] <= budgets[0]);
}

#[tokio::test]
async fn unreadable_documents_share_one_stable_reason() {
    for body in [
        "   ".to_owned(),
        "proxies: [".to_owned(),
        r#"{"items":[1,2,3]}"#.to_owned(),
        base64::engine::general_purpose::STANDARD.encode("proxies: ["),
        "proxies:\n  value: 1\n".to_owned(),
    ] {
        let source = fixtures();
        source.document(MAIN_URL, body.clone());
        let error = preview_error(source, MAIN_URL).await;
        assert_eq!(error, SubscriptionPreviewError::Unreadable, "{body}");
        assert_eq!(error.reason(), "subscription_unreadable", "{body}");
    }
}

#[tokio::test]
async fn source_and_download_failures_map_to_stable_reasons() {
    let cases = [
        (
            "sub.example.com/main.yaml".to_owned(),
            SubscriptionFetchError::Failed,
            SubscriptionPreviewError::SourceInvalid,
        ),
        (
            "http://sub.example.com/main.yaml".to_owned(),
            SubscriptionFetchError::Failed,
            SubscriptionPreviewError::TargetRejected,
        ),
        (
            MAIN_URL.to_owned(),
            SubscriptionFetchError::TooLarge,
            SubscriptionPreviewError::TooLarge,
        ),
        (
            MAIN_URL.to_owned(),
            SubscriptionFetchError::Timeout,
            SubscriptionPreviewError::Timeout,
        ),
        (
            MAIN_URL.to_owned(),
            SubscriptionFetchError::TooManyRedirects,
            SubscriptionPreviewError::TooManyRedirects,
        ),
        (
            MAIN_URL.to_owned(),
            SubscriptionFetchError::Failed,
            SubscriptionPreviewError::FetchFailed,
        ),
        (
            MAIN_URL.to_owned(),
            SubscriptionFetchError::TargetRejected,
            SubscriptionPreviewError::TargetRejected,
        ),
    ];
    for (url, failure, expected) in cases {
        let source = fixtures();
        source.failure(MAIN_URL, failure);
        let actual = preview_error(source, &url).await;
        assert_eq!(actual, expected, "{url} {failure}");
    }
    for error in [
        SubscriptionPreviewError::SourceInvalid,
        SubscriptionPreviewError::TargetRejected,
        SubscriptionPreviewError::Unreadable,
        SubscriptionPreviewError::TooLarge,
        SubscriptionPreviewError::Timeout,
        SubscriptionPreviewError::TooManyRedirects,
        SubscriptionPreviewError::FetchFailed,
    ] {
        assert!(error.reason().starts_with("subscription_"));
        assert!(error.to_string().is_ascii());
    }
}

#[tokio::test]
async fn preview_policy_rejects_plain_http_for_public_hosts() {
    assert_eq!(
        SubscriptionFetchPolicy::default().resolve_source("http://public.example.com/sub.yaml"),
        Err(SubscriptionSourceError::TargetRejected)
    );
}

#[tokio::test]
async fn malformed_connections_stay_rejected_while_valid_native_nodes_remain_available() {
    let source = fixtures();
    source.document(
        MAIN_URL,
        concat!(
            "http://#bad-http\n",
            "socks5://#bad-socks\n",
            "vmess://!!!!#bad-vmess\n",
            "ss://garbage#bad-ss\n",
            "http://192.0.2.11:8080#valid-http\n",
        ),
    );
    let result = preview(source, MAIN_URL).await;
    assert_eq!((result.accepted, result.rejected), (1, 4));
    assert!(
        result.candidates[..4]
            .iter()
            .all(|node| !node.parse_valid && !node.supported)
    );
    assert!(result.candidates[4].parse_valid);
    assert_eq!(result.diagnostics.len(), 4);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|item| item.code.as_str() == "node_parse_error")
    );

    let source = fixtures();
    source.document(MAIN_URL, r#"{"proxies":[
        {"name":"missing-server","type":"http"},
        {"name":"bad-port","type":"socks5","server":"192.0.2.1","port":70000},
        {"name":"bad-id","type":"vmess","server":"192.0.2.2","port":443,"uuid":"bad"},
        {"name":"valid-vless","type":"vless","server":"192.0.2.3","port":443,"uuid":"11111111-1111-1111-1111-111111111111"}
    ]}"#);
    let result = preview(source, MAIN_URL).await;
    assert_eq!(result.accepted, 0);
    assert!(result.candidates[..3].iter().all(|node| !node.parse_valid));
    assert!(result.candidates[3].parse_valid);
    assert!(!result.candidates[3].supported);
}

#[tokio::test]
async fn base64_wrappers_follow_the_decoded_document_instead_of_a_minimum_length() {
    for inner in [
        "http://a:80#n".to_owned(),
        "proxies: []".to_owned(),
        format!("http://a:80#n\n{}", " ".repeat(1_700_000)),
    ] {
        let source = fixtures();
        source.document(
            MAIN_URL,
            base64::engine::general_purpose::STANDARD.encode(&inner),
        );
        let result = preview(source, MAIN_URL).await;
        assert_eq!(result.format, SubscriptionFormat::Base64Document);
        assert_eq!(result.accepted, usize::from(inner.starts_with("http")));
    }
}

#[tokio::test]
async fn structured_syntax_errors_keep_the_document_error() {
    for body in [
        "proxies: [\nhttp://192.0.2.11:80#hidden",
        "{\"proxies\":[\nhttp://192.0.2.11:80#hidden",
    ] {
        let source = fixtures();
        source.document(MAIN_URL, body);
        assert_eq!(
            preview_error(source, MAIN_URL).await,
            SubscriptionPreviewError::Unreadable
        );
    }
    let source = fixtures();
    source.document(MAIN_URL, "broken-line\nhttp://192.0.2.11:80#valid");
    let result = preview(source, MAIN_URL).await;
    assert_eq!((result.accepted, result.rejected), (1, 1));
}

#[tokio::test]
async fn skipped_provider_types_and_shapes_produce_review_diagnostics() {
    for (body, code) in [
        (
            r#"{"proxy-providers":{"local":{"type":"file","path":"/etc/passwd"}}}"#,
            "provider_unsupported",
        ),
        (
            r#"{"proxy-providers":{"local":{"type":"inline","payload":[]}}}"#,
            "provider_unsupported",
        ),
        (r#"{"proxy-providers":["broken"]}"#, "provider_parse_error"),
        (
            r#"{"proxy-providers":{"remote":{"type":"http"}}}"#,
            "provider_parse_error",
        ),
    ] {
        let source = fixtures();
        source.document(MAIN_URL, body);
        let result = preview(source, MAIN_URL).await;
        assert!(result.candidates.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code.as_str(), code);
    }
}

#[tokio::test]
async fn preview_capacity_applies_to_nodes_diagnostics_and_combined_providers() {
    use gateway_admin::model::subscriptions::MAX_PREVIEW_CANDIDATES;
    let node = serde_json::json!({"name":"node", "type":"http", "server":"192.0.2.1", "port":8080});
    let source = fixtures();
    source.document(
        MAIN_URL,
        serde_json::json!({"proxies": vec![node.clone(); MAX_PREVIEW_CANDIDATES + 1]}).to_string(),
    );
    assert_eq!(
        preview_error(source, MAIN_URL).await,
        SubscriptionPreviewError::PreviewTooLarge
    );
    let source = fixtures();
    source.document(
        MAIN_URL,
        serde_json::json!({"proxies": vec![node.clone(); MAX_PREVIEW_CANDIDATES],
        "proxy-providers":{"remote":{"type":"http","url":PROVIDER_URL}}})
        .to_string(),
    );
    source.document(
        PROVIDER_URL,
        serde_json::json!({"proxies":[node]}).to_string(),
    );
    assert_eq!(
        preview_error(source, MAIN_URL).await,
        SubscriptionPreviewError::PreviewTooLarge
    );
}

#[tokio::test]
async fn yaml_flow_maps_and_uri_scheme_case_preserve_supported_syntax() {
    let source = fixtures();
    source.document(
        MAIN_URL,
        "{proxies: [{name: flow, type: http, server: example.com, port: 80}]}",
    );
    let result = preview(source, MAIN_URL).await;
    assert_eq!(result.format, SubscriptionFormat::ClashYaml);
    assert_eq!(result.accepted, 1);
    let source = fixtures();
    source.document(MAIN_URL, "proxies : [\nhttp://example.com:80#hidden");
    assert_eq!(
        preview_error(source, MAIN_URL).await,
        SubscriptionPreviewError::Unreadable
    );
    let source = fixtures();
    source.document(
        MAIN_URL,
        "HTTPS://example.com/#tls\nSOCKS5H://example.com:1080#socks",
    );
    let result = preview(source, MAIN_URL).await;
    assert_eq!(result.accepted, 2);
}
