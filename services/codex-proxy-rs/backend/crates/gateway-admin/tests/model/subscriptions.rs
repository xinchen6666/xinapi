//! 验证订阅下载策略的源地址校验、测试源允许列表与配置校验

use gateway_admin::SubscriptionConfig;
use gateway_admin::model::subscriptions::{SubscriptionFetchPolicy, SubscriptionSourceError};

fn test_policy() -> SubscriptionFetchPolicy {
    SubscriptionFetchPolicy::new([" 127.0.0.1 ".to_owned(), "LOCALHOST".to_owned()])
}

#[test]
fn policy_accepts_https_for_any_host() {
    let policy = SubscriptionFetchPolicy::default();
    let url = policy
        .resolve_source("https://sub.example.com/clash.yaml")
        .expect("https source");
    assert_eq!(url.host_str(), Some("sub.example.com"));
}

#[test]
fn policy_rejects_plain_http_outside_the_test_host_list() {
    let policy = SubscriptionFetchPolicy::default();
    assert_eq!(
        policy.resolve_source("http://sub.example.com/clash.yaml"),
        Err(SubscriptionSourceError::TargetRejected)
    );
}

#[test]
fn policy_accepts_plain_http_only_for_configured_test_hosts() {
    let policy = test_policy();
    assert!(
        policy
            .resolve_source("http://127.0.0.1:18080/fixture.yaml")
            .is_ok()
    );
    assert!(
        policy
            .resolve_source("http://localhost:18080/fixture.yaml")
            .is_ok()
    );
    assert_eq!(
        policy.allow("http", "192.0.2.10"),
        Err(SubscriptionSourceError::TargetRejected)
    );
}

#[test]
fn policy_rejects_schemes_outside_http_and_https() {
    let policy = test_policy();
    assert_eq!(
        policy.allow("file", "127.0.0.1"),
        Err(SubscriptionSourceError::TargetRejected)
    );
    assert_eq!(
        policy.allow("ftp", "127.0.0.1"),
        Err(SubscriptionSourceError::TargetRejected)
    );
}

#[test]
fn policy_rejects_relative_empty_and_oversized_sources() {
    let policy = test_policy();
    for value in [
        "",
        "   ",
        "/subscription.yaml",
        "sub.example.com/clash.yaml",
    ] {
        assert_eq!(
            policy.resolve_source(value),
            Err(SubscriptionSourceError::Malformed),
            "{value}"
        );
    }
    let oversized = format!("https://sub.example.com/{}", "a".repeat(2048));
    assert_eq!(
        policy.resolve_source(&oversized),
        Err(SubscriptionSourceError::Malformed)
    );
}

#[test]
fn policy_rejects_control_characters_in_sources() {
    let policy = test_policy();
    assert_eq!(
        policy.resolve_source("https://sub.example.com/clash.yaml\nHost: evil"),
        Err(SubscriptionSourceError::Malformed)
    );
}

#[test]
fn subscription_config_normalizes_test_hosts_and_keeps_defaults_empty() {
    let default = SubscriptionConfig::default();
    assert!(default.insecure_test_hosts.is_empty());
    assert_eq!(default.policy(), SubscriptionFetchPolicy::default());

    let mut config = SubscriptionConfig {
        insecure_test_hosts: vec![" 127.0.0.1 ".to_owned(), "LocalHost".to_owned()],
    };
    config
        .resolve_and_validate()
        .expect("valid subscription test hosts");
    assert_eq!(
        config.policy(),
        SubscriptionFetchPolicy::new(["127.0.0.1".to_owned(), "localhost".to_owned()])
    );
}

#[test]
fn subscription_config_rejects_hosts_carrying_scheme_port_or_path() {
    for host in [
        "",
        "https://sub.example.com",
        "sub.example.com:8080",
        "sub/example",
    ] {
        let mut config = SubscriptionConfig {
            insecure_test_hosts: vec![host.to_owned()],
        };
        assert!(config.resolve_and_validate().is_err(), "{host}");
    }
}
