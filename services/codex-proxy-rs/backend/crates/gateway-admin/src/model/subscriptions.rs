//! 订阅预览的格式、协议、诊断、结果与下载策略值对象
//!
//! 公开预览事实与私有连接文档分别投影，连接秘密只供服务端持久化和匹配

use std::time::Duration;

use url::Url;

/// 主文档原始正文上限，2 MiB
pub const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;

/// 解压后正文上限，8 MiB
pub const MAX_DECOMPRESSED_BYTES: usize = 8 * 1024 * 1024;

/// 单次预览的总时限，主文档与全部 provider 文档共用这一预算
pub const FETCH_BUDGET: Duration = Duration::from_secs(10);

/// 单次下载允许跟随的跳转次数
pub const MAX_REDIRECTS: usize = 3;

/// 节点展示名上限，单条记录由此限制响应放大
pub const MAX_NODE_NAME_CHARS: usize = 256;

/// 订阅源地址长度上限
pub const MAX_SOURCE_URL_BYTES: usize = 2048;

/// HTTP 型 provider 允许的最大数量，避免一份文档放大下载次数
pub const MAX_PROVIDERS: usize = 32;

/// 整次预览的节点与诊断预算，合并 provider 时沿用同一上限
pub const MAX_PREVIEW_CANDIDATES: usize = 4096;
pub const MAX_PREVIEW_DIAGNOSTICS: usize = 4096;

/// 与管理适配层接收能力一致的完整 JSON 响应预算
pub const MAX_PREVIEW_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

/// 订阅正文的实际格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionFormat {
    ClashYaml,
    ClashJson,
    UriList,
    Base64Document,
}

impl SubscriptionFormat {
    /// 稳定 wire 名称
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClashYaml => "clash_yaml",
            Self::ClashJson => "clash_json",
            Self::UriList => "uri_list",
            Self::Base64Document => "base64_document",
        }
    }
}

/// 逐节点识别的协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeProtocol {
    Ss,
    Vmess,
    Vless,
    Trojan,
    Hysteria2,
    Tuic,
    Socks5,
    Http,
    Unknown,
}

impl NodeProtocol {
    /// 稳定 wire 名称
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ss => "ss",
            Self::Vmess => "vmess",
            Self::Vless => "vless",
            Self::Trojan => "trojan",
            Self::Hysteria2 => "hysteria2",
            Self::Tuic => "tuic",
            Self::Socks5 => "socks5",
            Self::Http => "http",
            Self::Unknown => "unknown",
        }
    }

    /// 可交由受管 Mihomo 执行的协议族
    /// 连接参数和传输组合在节点验证时按内核合同逐项检查
    #[must_use]
    pub const fn is_materializable(self) -> bool {
        matches!(
            self,
            Self::Http | Self::Socks5 | Self::Ss | Self::Vmess | Self::Vless | Self::Trojan
        )
    }
}

/// 节点级与 provider 级诊断的稳定码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    UnsupportedProtocol,
    NodeParseError,
    ProviderFetchFailed,
    ProviderDepthLimit,
    ProviderUnsupported,
    ProviderParseError,
}

impl DiagnosticCode {
    /// 稳定 wire 名称
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedProtocol => "unsupported_protocol",
            Self::NodeParseError => "node_parse_error",
            Self::ProviderFetchFailed => "provider_fetch_failed",
            Self::ProviderDepthLimit => "provider_depth_limit",
            Self::ProviderUnsupported => "provider_unsupported",
            Self::ProviderParseError => "provider_parse_error",
        }
    }
}

/// 单条诊断；subject 是节点名、provider 名或字段名，取不到时为空字符串
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewDiagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub subject: String,
}

impl PreviewDiagnostic {
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            subject: subject.into(),
        }
    }
}

/// 一个节点的可公开事实
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeCandidate {
    pub name: String,
    pub protocol: NodeProtocol,
    pub parse_valid: bool,
    pub supported: bool,
    pub connection: Option<super::subscription_sources::ConnectionDocument>,
}

impl NodeCandidate {
    #[must_use]
    pub fn new(name: impl Into<String>, protocol: NodeProtocol) -> Self {
        Self {
            name: name.into(),
            protocol,
            parse_valid: protocol != NodeProtocol::Unknown,
            supported: protocol.is_materializable(),
            connection: None,
        }
    }
}

/// 一次预览的结果；accepted 与 rejected 之和等于 candidates 长度
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionPreview {
    pub source_url: String,
    pub format: SubscriptionFormat,
    pub fetched_bytes: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub candidates: Vec<NodeCandidate>,
    pub diagnostics: Vec<PreviewDiagnostic>,
}

/// 预览失败的稳定分类；每类对应一条脱敏英文说明与一个稳定机器码
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionPreviewError {
    #[error("Provide an absolute http or https subscription URL.")]
    SourceInvalid,
    #[error("Use https, or an http host listed in the subscription test hosts.")]
    TargetRejected,
    #[error("The subscription document is unreadable as Clash YAML, Clash JSON or a URI list.")]
    Unreadable,
    #[error("The subscription document exceeds the size limit.")]
    TooLarge,
    #[error("The subscription preview exceeds the result capacity.")]
    PreviewTooLarge,
    #[error("The subscription download exceeded the time limit.")]
    Timeout,
    #[error("The subscription download exceeded the redirect limit.")]
    TooManyRedirects,
    #[error("The subscription download failed.")]
    FetchFailed,
}

impl SubscriptionPreviewError {
    /// 稳定机器码，调用方按它分流，不用文案文本
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::SourceInvalid => "subscription_source_invalid",
            Self::TargetRejected => "subscription_target_not_allowed",
            Self::Unreadable => "subscription_unreadable",
            Self::TooLarge => "subscription_too_large",
            Self::PreviewTooLarge => "subscription_preview_too_large",
            Self::Timeout => "subscription_timeout",
            Self::TooManyRedirects => "subscription_too_many_redirects",
            Self::FetchFailed => "subscription_fetch_failed",
        }
    }

    /// 映射到管理用例的错误分类；正文内容类失败按 422 返回
    #[must_use]
    pub fn into_admin_error(self) -> crate::model::AdminError {
        let message = self.to_string();
        match self {
            Self::TargetRejected => crate::model::AdminError::bad_request(message),
            Self::Unreadable | Self::TooLarge | Self::PreviewTooLarge => {
                crate::model::AdminError::invalid(message)
            }
            Self::Timeout | Self::TooManyRedirects | Self::FetchFailed => {
                crate::model::AdminError::bad_gateway(message)
            }
            Self::SourceInvalid => crate::model::AdminError::invalid(message),
        }
    }
}

/// 源地址形状错误或目标被策略拒绝
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionSourceError {
    /// 地址缺少主机、长度超限或带有控制字符
    #[error("subscription source URL shape is invalid")]
    Malformed,
    /// scheme 与主机不满足下载策略
    #[error("subscription target scheme or host is rejected")]
    TargetRejected,
}

/// 订阅下载目标策略
///
/// 默认只接受 https；明文 http 仅向配置列出的测试源主机开放，
/// 跳转的每一跳都按同一规则复核
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubscriptionFetchPolicy {
    insecure_test_hosts: Vec<String>,
}

impl SubscriptionFetchPolicy {
    /// 由配置的测试源主机构造策略；主机名统一小写并丢弃空项
    #[must_use]
    pub fn new(hosts: impl IntoIterator<Item = String>) -> Self {
        let insecure_test_hosts = hosts
            .into_iter()
            .map(|host| host.trim().to_ascii_lowercase())
            .filter(|host| !host.is_empty())
            .collect();
        Self {
            insecure_test_hosts,
        }
    }

    /// 校验管理员提交的源地址并返回解析后的绝对地址
    ///
    /// # Errors
    ///
    /// 地址形状或下载策略不满足时返回 [`SubscriptionSourceError`]
    pub fn resolve_source(&self, value: &str) -> Result<Url, SubscriptionSourceError> {
        let trimmed = value.trim();
        if trimmed.is_empty()
            || trimmed.len() > MAX_SOURCE_URL_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(SubscriptionSourceError::Malformed);
        }
        let url = Url::parse(trimmed).map_err(|_| SubscriptionSourceError::Malformed)?;
        let host = url.host_str().ok_or(SubscriptionSourceError::Malformed)?;
        self.allow(url.scheme(), host)?;
        Ok(url)
    }

    /// 复核单个下载目标；跳转目标同样经过这里
    ///
    /// # Errors
    ///
    /// scheme 属于 http(s) 之外，或明文 http 的主机未列入测试源时拒绝
    pub fn allow(&self, scheme: &str, host: &str) -> Result<(), SubscriptionSourceError> {
        let host = host.trim().to_ascii_lowercase();
        if host.is_empty() {
            return Err(SubscriptionSourceError::TargetRejected);
        }
        match scheme.to_ascii_lowercase().as_str() {
            "https" => Ok(()),
            "http" if self.insecure_test_hosts.iter().any(|entry| entry == &host) => Ok(()),
            _ => Err(SubscriptionSourceError::TargetRejected),
        }
    }
}

/// 订阅下载相关的启动配置
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct SubscriptionConfig {
    /// 允许以明文 http 下载的测试源主机；默认空，仅本地测试源使用
    #[serde(default)]
    pub insecure_test_hosts: Vec<String>,
}

impl SubscriptionConfig {
    /// 校验测试源主机名；这里只做形状检查，不做解析或连通性探测
    ///
    /// # Errors
    ///
    /// 主机名带有 scheme、端口、路径、空白或控制字符时返回错误
    pub fn resolve_and_validate(&mut self) -> Result<(), crate::AdminConfigError> {
        let mut normalized = Vec::with_capacity(self.insecure_test_hosts.len());
        for host in &self.insecure_test_hosts {
            let host = host.trim();
            if host.is_empty()
                || host.len() > 253
                || host.chars().any(|value| {
                    !(value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_'))
                })
            {
                return Err(crate::AdminConfigError::InvalidField(
                    "subscriptions.insecure_test_hosts",
                ));
            }
            normalized.push(host.to_ascii_lowercase());
        }
        self.insecure_test_hosts = normalized;
        Ok(())
    }

    /// 由配置构造下载策略
    #[must_use]
    pub fn policy(&self) -> SubscriptionFetchPolicy {
        SubscriptionFetchPolicy::new(self.insecure_test_hosts.clone())
    }
}
