//! 订阅文档的受限下载：逐跳审核并固定连接地址，限制正文与解压规模
//!
//! 请求只带固定协商头，跳转由本模块自行跟随；DNS 与各跳下载共用总时限

use std::{io::Read as _, net::SocketAddr, time::Duration};

use async_trait::async_trait;
use gateway_admin::{
    model::subscriptions::{
        MAX_DECOMPRESSED_BYTES, MAX_DOCUMENT_BYTES, MAX_REDIRECTS, SubscriptionFetchPolicy,
    },
    ports::subscription::{FetchedDocument, SubscriptionDocumentSource, SubscriptionFetchError},
};
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, LOCATION};

use crate::outbound::NetworkPolicy;

/// 每跳独立解析与连接的订阅下载源
pub struct HttpSubscriptionSource;

impl HttpSubscriptionSource {
    /// 在启动时校验 TLS 与客户端配置，各跳连接由专用客户端持有
    ///
    /// # Errors
    ///
    /// TLS 后端或客户端构造失败时返回错误
    pub fn new() -> Result<Self, reqwest::Error> {
        Self::client_builder().build()?;
        Ok(Self)
    }

    fn client_builder() -> reqwest::ClientBuilder {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            // 原始传输与解压后正文分别计量，始终由 read_document 承担解压
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
    }

    async fn fetch_within(
        &self,
        url: &str,
        policy: &SubscriptionFetchPolicy,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        let mut target =
            reqwest::Url::parse(url).map_err(|_| SubscriptionFetchError::TargetRejected)?;
        let public_network = NetworkPolicy::default();
        let mut redirects = 0usize;
        loop {
            let host = target
                .host_str()
                .ok_or(SubscriptionFetchError::TargetRejected)?
                .to_owned();
            policy
                .allow(target.scheme(), &host)
                .map_err(|_| SubscriptionFetchError::TargetRejected)?;
            // 明文 HTTP 通过策略校验即代表精确命中的测试源，HTTPS 始终要求公网地址
            let test_http_source = target.scheme() == "http";
            let port = target
                .port_or_known_default()
                .ok_or(SubscriptionFetchError::TargetRejected)?;
            let dns_host = host.trim_matches(['[', ']']);
            // 地址字面量直接使用审核值；域名只在这里解析，全部候选通过后才建立连接
            let mut addresses = if let Ok(ip) = dns_host.parse() {
                vec![SocketAddr::new(ip, port)]
            } else {
                tokio::net::lookup_host((dns_host, port))
                    .await
                    .map_err(|_| SubscriptionFetchError::Failed)?
                    .collect::<Vec<_>>()
            };
            if addresses.is_empty() {
                return Err(SubscriptionFetchError::Failed);
            }
            if addresses.iter().any(|address| {
                address.port() != port
                    || (!test_http_source && !public_network.permits(address.ip()))
            }) {
                return Err(SubscriptionFetchError::TargetRejected);
            }
            addresses.sort_unstable();
            addresses.dedup();
            // 每跳剥离地址凭据，固定解析结果；原域名继续负责 Host、TLS SNI 与证书校验
            target
                .set_username("")
                .map_err(|()| SubscriptionFetchError::TargetRejected)?;
            target
                .set_password(None)
                .map_err(|()| SubscriptionFetchError::TargetRejected)?;
            let client = Self::client_builder()
                .resolve_to_addrs(dns_host, &addresses)
                .build()
                .map_err(|_| SubscriptionFetchError::Failed)?;
            let mut response = client
                .get(target.clone())
                .header(ACCEPT, "*/*")
                .header(ACCEPT_ENCODING, "gzip, deflate")
                .send()
                .await
                .map_err(|_| SubscriptionFetchError::Failed)?;
            // 可观察的实际对端也应属于本跳已审核集合，映射 IPv4 采用统一比较形式
            if let Some(peer) = response.remote_addr()
                && !addresses.iter().any(|address| {
                    address.port() == peer.port()
                        && address.ip().to_canonical() == peer.ip().to_canonical()
                })
            {
                return Err(SubscriptionFetchError::TargetRejected);
            }
            let status = response.status();
            if status.is_redirection() {
                if redirects >= MAX_REDIRECTS {
                    return Err(SubscriptionFetchError::TooManyRedirects);
                }
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(SubscriptionFetchError::Failed)?;
                target = target
                    .join(location)
                    .map_err(|_| SubscriptionFetchError::Failed)?;
                redirects += 1;
                continue;
            }
            if !status.is_success() {
                return Err(SubscriptionFetchError::Failed);
            }
            return read_document(&mut response).await;
        }
    }
}

#[async_trait]
impl SubscriptionDocumentSource for HttpSubscriptionSource {
    async fn fetch(
        &self,
        url: &str,
        policy: &SubscriptionFetchPolicy,
        budget: Duration,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        if budget.is_zero() {
            return Err(SubscriptionFetchError::Timeout);
        }
        // 时限覆盖 DNS、连接、响应头、正文与全部跳转，调用方传入整次预览的剩余预算
        tokio::time::timeout(budget, self.fetch_within(url, policy))
            .await
            .map_err(|_| SubscriptionFetchError::Timeout)?
    }
}

async fn read_document(
    response: &mut reqwest::Response,
) -> Result<FetchedDocument, SubscriptionFetchError> {
    // 先按声明长度快速拒绝，最终以实际读到的字节为准
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOCUMENT_BYTES as u64)
    {
        return Err(SubscriptionFetchError::TooLarge);
    }
    let encoding = response
        .headers()
        .get(CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let mut raw = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| SubscriptionFetchError::Failed)?
    {
        if raw.len() + chunk.len() > MAX_DOCUMENT_BYTES {
            return Err(SubscriptionFetchError::TooLarge);
        }
        raw.extend_from_slice(&chunk);
    }
    let body = match encoding.as_str() {
        "" | "identity" => raw,
        "gzip" => inflate(flate2::read::GzDecoder::new(raw.as_slice()))?,
        "deflate" => inflate(flate2::read::ZlibDecoder::new(raw.as_slice()))?,
        // 其余编码说明服务器忽略了协商结果，按下载失败收敛
        _ => return Err(SubscriptionFetchError::Failed),
    };
    Ok(FetchedDocument { body })
}

fn inflate(decoder: impl std::io::Read) -> Result<Vec<u8>, SubscriptionFetchError> {
    let limit = u64::try_from(MAX_DECOMPRESSED_BYTES + 1).unwrap_or(u64::MAX);
    let mut output = Vec::new();
    decoder
        .take(limit)
        .read_to_end(&mut output)
        .map_err(|_| SubscriptionFetchError::Failed)?;
    if output.len() > MAX_DECOMPRESSED_BYTES {
        return Err(SubscriptionFetchError::TooLarge);
    }
    Ok(output)
}
