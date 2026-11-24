//! 验证订阅下载的目标复核、跳转上限、正文与解压规模、时间预算与失败收敛

use std::time::Duration;

use flate2::{
    Compression,
    write::{GzEncoder, ZlibEncoder},
};
use gateway_admin::model::subscriptions::{
    MAX_DECOMPRESSED_BYTES, MAX_DOCUMENT_BYTES, SubscriptionFetchPolicy,
};
use gateway_admin::ports::subscription::{SubscriptionDocumentSource, SubscriptionFetchError};
use gateway_host::subscription_fetch::HttpSubscriptionSource;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

fn source() -> HttpSubscriptionSource {
    HttpSubscriptionSource::new().expect("subscription source")
}

fn local_policy() -> SubscriptionFetchPolicy {
    SubscriptionFetchPolicy::new(["127.0.0.1".to_owned(), "localhost".to_owned()])
}

fn strict_policy() -> SubscriptionFetchPolicy {
    SubscriptionFetchPolicy::default()
}

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, body).expect("gzip fixture");
    encoder.finish().expect("gzip fixture")
}

fn deflate(body: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, body).expect("deflate fixture");
    encoder.finish().expect("deflate fixture")
}

const BUDGET: Duration = Duration::from_secs(5);

#[tokio::test]
async fn fetch_reads_an_allowlisted_plain_http_document() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sub.yaml"))
        .respond_with(ResponseTemplate::new(200).set_body_string("proxies: []\n"))
        .mount(&server)
        .await;

    let document = source()
        .fetch(
            &format!("{}/sub.yaml", server.uri()),
            &local_policy(),
            BUDGET,
        )
        .await
        .expect("fetched document");

    assert_eq!(document.body, b"proxies: []\n");
}

#[tokio::test]
async fn fetch_rejects_plain_http_hosts_outside_the_test_policy() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let error = source()
        .fetch(
            &format!("{}/sub.yaml", server.uri()),
            &strict_policy(),
            BUDGET,
        )
        .await
        .expect_err("rejected target");

    assert_eq!(error, SubscriptionFetchError::TargetRejected);
    server.verify().await;
}

#[tokio::test]
async fn fetch_decompresses_gzip_and_deflate_bodies() {
    let server = MockServer::start().await;
    let body = b"proxies:\n  - {name: alpha}\n".to_vec();
    Mock::given(method("GET"))
        .and(path("/gzip"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_bytes(gzip(&body)),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/deflate"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "deflate")
                .set_body_bytes(deflate(&body)),
        )
        .mount(&server)
        .await;

    for route in ["/gzip", "/deflate"] {
        let document = source()
            .fetch(&format!("{}{route}", server.uri()), &local_policy(), BUDGET)
            .await
            .expect("fetched document");
        assert_eq!(document.body, body, "{route}");
    }
}

#[tokio::test]
async fn fetch_limits_the_raw_and_decompressed_body() {
    let server = MockServer::start().await;
    let oversized = vec![b'a'; MAX_DOCUMENT_BYTES + 1];
    Mock::given(method("GET"))
        .and(path("/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(oversized))
        .mount(&server)
        .await;
    let inflated = vec![b'a'; MAX_DECOMPRESSED_BYTES + 1];
    Mock::given(method("GET"))
        .and(path("/inflated"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-encoding", "gzip")
                .set_body_bytes(gzip(&inflated)),
        )
        .mount(&server)
        .await;

    for route in ["/raw", "/inflated"] {
        let error = source()
            .fetch(&format!("{}{route}", server.uri()), &local_policy(), BUDGET)
            .await
            .expect_err("oversized body");
        assert_eq!(error, SubscriptionFetchError::TooLarge, "{route}");
    }
}

#[tokio::test]
async fn fetch_follows_allowlisted_redirects_up_to_the_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hop-1"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/hop-2"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/hop-2"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/hop-3"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/hop-3"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/hop-4"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/hop-4"))
        .respond_with(ResponseTemplate::new(200).set_body_string("proxies: []\n"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/loop"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/loop"))
        .mount(&server)
        .await;

    let document = source()
        .fetch(&format!("{}/hop-1", server.uri()), &local_policy(), BUDGET)
        .await
        .expect("redirect chain");
    assert_eq!(document.body, b"proxies: []\n");

    let error = source()
        .fetch(&format!("{}/loop", server.uri()), &local_policy(), BUDGET)
        .await
        .expect_err("redirect loop");
    assert_eq!(error, SubscriptionFetchError::TooManyRedirects);
}

#[tokio::test]
async fn fetch_rejects_redirect_targets_outside_the_policy() {
    let server = MockServer::start().await;
    for route in ["/to-https", "/to-file", "/to-other-host"] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(302).insert_header(
                "location",
                match route {
                    "/to-https" => "https://127.0.0.1:1/main.yaml",
                    "/to-file" => "file:///etc/passwd",
                    _ => "http://192.0.2.10/main.yaml",
                },
            ))
            .mount(&server)
            .await;
    }

    // 首跳使用测试源策略，跳转目标不满足 https 与允许列表时立即收敛
    for route in ["/to-file", "/to-other-host"] {
        let error = source()
            .fetch(&format!("{}{route}", server.uri()), &local_policy(), BUDGET)
            .await
            .expect_err("rejected redirect");
        assert_eq!(error, SubscriptionFetchError::TargetRejected, "{route}");
    }

    // https 目标通过策略复核后因本机端口拒绝连接收敛为下载失败
    let error = source()
        .fetch(
            &format!("{}/to-https", server.uri()),
            &local_policy(),
            BUDGET,
        )
        .await
        .expect_err("unreachable https target");
    assert_eq!(error, SubscriptionFetchError::Failed);
}

#[tokio::test]
async fn fetch_reports_timeout_within_the_given_budget() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(3))
                .set_body_string("proxies: []\n"),
        )
        .mount(&server)
        .await;

    let error = source()
        .fetch(
            &format!("{}/slow", server.uri()),
            &local_policy(),
            Duration::from_millis(200),
        )
        .await
        .expect_err("timeout");
    assert_eq!(error, SubscriptionFetchError::Timeout);

    let error = source()
        .fetch(
            &format!("{}/slow", server.uri()),
            &local_policy(),
            Duration::ZERO,
        )
        .await
        .expect_err("exhausted budget");
    assert_eq!(error, SubscriptionFetchError::Timeout);
}

#[tokio::test]
async fn fetch_reports_http_failures_as_download_failures() {
    let server = MockServer::start().await;
    for status in [404, 500] {
        Mock::given(method("GET"))
            .and(path(format!("/status-{status}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
    }

    for status in [404, 500] {
        let error = source()
            .fetch(
                &format!("{}/status-{status}", server.uri()),
                &local_policy(),
                BUDGET,
            )
            .await
            .expect_err("http failure");
        assert_eq!(error, SubscriptionFetchError::Failed, "{status}");
    }
}
