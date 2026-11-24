//! 验证订阅预览接口的固定字段集、部分成功结果与脱敏错误码

use std::{collections::BTreeMap, sync::Mutex};

use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use gateway_admin::{
    model::subscriptions::SubscriptionFetchPolicy,
    ports::subscription::{FetchedDocument, SubscriptionDocumentSource, SubscriptionFetchError},
};
use serde_json::{Value, json};
use tower::ServiceExt as _;

use super::{AdminTestFixture, AdminTestState};

const SOURCE_URL: &str = "https://sub.example.com/main.yaml";

#[derive(Default)]
pub(crate) struct TestSubscriptionSource {
    documents: Mutex<BTreeMap<String, Result<Vec<u8>, SubscriptionFetchError>>>,
}

impl TestSubscriptionSource {
    pub(crate) fn document(&self, url: &str, body: impl Into<Vec<u8>>) {
        self.documents
            .lock()
            .expect("test subscription documents")
            .insert(url.to_owned(), Ok(body.into()));
    }

    pub(crate) fn failure(&self, url: &str, error: SubscriptionFetchError) {
        self.documents
            .lock()
            .expect("test subscription documents")
            .insert(url.to_owned(), Err(error));
    }
}

#[async_trait]
impl SubscriptionDocumentSource for TestSubscriptionSource {
    async fn fetch(
        &self,
        url: &str,
        _: &SubscriptionFetchPolicy,
        _: std::time::Duration,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        match self
            .documents
            .lock()
            .expect("test subscription documents")
            .get(url)
        {
            Some(Ok(body)) => Ok(FetchedDocument { body: body.clone() }),
            Some(Err(error)) => Err(*error),
            None => Err(SubscriptionFetchError::Failed),
        }
    }
}

async fn request(
    fixture: &AdminTestFixture,
    body: Option<Value>,
    authenticated: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .header("x-request-id", "req_subscription_tests")
        .uri("/api/admin/subscriptions/preview")
        .method("POST");
    if authenticated {
        builder = builder.header(header::COOKIE, "cpr_session=valid-session");
    }
    let body = body.map_or_else(Body::empty, |body| Body::from(body.to_string()));
    let response = gateway_api::admin::subscriptions::router::<AdminTestState>()
        .with_state(fixture.state())
        .oneshot(
            builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(body)
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let text = std::str::from_utf8(&body).unwrap();
    // 上游正文、节点端口与凭据都留在服务端
    for secret in ["198.51.100.", "aes-128-gcm", "secret-password"] {
        assert!(!text.contains(secret), "{secret} leaked: {text}");
    }
    (status, serde_json::from_slice(&body).unwrap())
}

#[tokio::test]
async fn preview_should_return_the_whitelisted_field_set() {
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");
    fixture.subscription_source.document(
        SOURCE_URL,
        concat!(
            "proxies:\n",
            "  - {name: alpha, type: socks5, server: 198.51.100.1, port: 1080}\n",
            "  - {name: beta, type: ss, server: 198.51.100.2, port: 8388, ",
            "cipher: aes-128-gcm, password: secret-password}\n",
        ),
    );

    let (status, body) = request(&fixture, Some(json!({"source_url": SOURCE_URL})), true).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["code"], 200);
    assert_eq!(body["message"], "OK");
    let data = body["data"].as_object().expect("preview data");
    let mut keys = data.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "accepted",
            "candidates",
            "diagnostics",
            "fetched_bytes",
            "format",
            "rejected",
            "source_url",
        ]
    );
    assert_eq!(data["source_url"], SOURCE_URL);
    assert_eq!(data["format"], "clash_yaml");
    assert_eq!(data["accepted"], 1);
    assert_eq!(data["rejected"], 1);
    assert!(
        data["fetched_bytes"]
            .as_u64()
            .is_some_and(|value| value > 0)
    );

    let candidates = data["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 2);
    for candidate in candidates {
        let mut keys = candidate
            .as_object()
            .expect("candidate")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, vec!["name", "parse_valid", "protocol", "supported"]);
    }
    assert_eq!(candidates[0]["name"], "alpha");
    assert_eq!(candidates[0]["protocol"], "socks5");
    assert_eq!(candidates[0]["supported"], true);
    assert_eq!(candidates[1]["protocol"], "ss");
    assert_eq!(candidates[1]["supported"], false);

    let diagnostics = data["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    let mut keys = diagnostics[0]
        .as_object()
        .expect("diagnostic")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(keys, vec!["code", "message", "subject"]);
    assert_eq!(diagnostics[0]["code"], "unsupported_protocol");
    assert_eq!(diagnostics[0]["subject"], "beta");
}

#[tokio::test]
async fn preview_should_keep_partial_results_for_unreadable_nodes() {
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");
    fixture.subscription_source.document(
        SOURCE_URL,
        "proxies:\n  - {name: alpha, type: socks5, server: 198.51.100.3, port: 1080}\n  - 42\n",
    );

    let (status, body) = request(&fixture, Some(json!({"source_url": SOURCE_URL})), true).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["accepted"], 1);
    assert_eq!(body["data"]["rejected"], 1);
    assert_eq!(body["data"]["diagnostics"][0]["code"], "node_parse_error");
}

#[tokio::test]
async fn unreadable_document_should_answer_422_with_the_stable_reason() {
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");
    fixture
        .subscription_source
        .document(SOURCE_URL, "proxies: [");

    let (status, body) = request(&fixture, Some(json!({"source_url": SOURCE_URL})), true).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40001);
    assert_eq!(body["reason"], "subscription_unreadable");
    assert_eq!(body["data"], Value::Null);
    assert!(body["message"].as_str().is_some_and(str::is_ascii));
}

#[tokio::test]
async fn download_failure_should_answer_502_with_the_stable_reason() {
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");
    fixture
        .subscription_source
        .failure(SOURCE_URL, SubscriptionFetchError::Timeout);

    let (status, body) = request(&fixture, Some(json!({"source_url": SOURCE_URL})), true).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body["code"], 50201);
    assert_eq!(body["reason"], "subscription_timeout");
}

#[tokio::test]
async fn plain_http_source_should_answer_400_with_the_target_reason() {
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");

    let (status, body) = request(
        &fixture,
        Some(json!({"source_url": "http://public.example.com/sub.yaml"})),
        true,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], 40001);
    assert_eq!(body["reason"], "subscription_target_not_allowed");
}

#[tokio::test]
async fn unknown_field_should_be_rejected_by_the_admin_envelope() {
    // 字段错误沿用管理端既有约定：HTTP 422 与业务码 40001
    let fixture = AdminTestFixture::new().await;
    fixture.auth.insert_session("valid-session");

    let (status, body) = request(
        &fixture,
        Some(json!({"source_url": SOURCE_URL, "format": "clash_yaml"})),
        true,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], 40001);
}

#[tokio::test]
async fn preview_should_require_an_admin_session() {
    let fixture = AdminTestFixture::new().await;

    let (status, body) = request(&fixture, Some(json!({"source_url": SOURCE_URL})), false).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], 40101);
}
