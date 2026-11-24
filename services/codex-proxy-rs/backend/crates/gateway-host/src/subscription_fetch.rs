//! 订阅文档的受限下载：逐跳复核目标、限制正文与解压规模并共用总时限
//!
//! 请求只带固定协商头，不带任何凭据或 Cookie；跳转由本模块自行跟随，
//! 以便每一跳都按管理端策略复核 scheme 与主机

use std::{io::Read as _, time::Duration};

use async_trait::async_trait;
use gateway_admin::{
    model::subscriptions::{
        MAX_DECOMPRESSED_BYTES, MAX_DOCUMENT_BYTES, MAX_REDIRECTS, SubscriptionFetchPolicy,
    },
    ports::subscription::{FetchedDocument, SubscriptionDocumentSource, SubscriptionFetchError},
};
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, LOCATION};

/// 订阅文档下载客户端
pub struct HttpSubscriptionSource {
    client: reqwest::Client,
}

impl HttpSubscriptionSource {
    /// 构造直连客户端：跳转自行处理，环境代理由部署出口决定，这里固定直连
    ///
    /// # Errors
    ///
    /// TLS 后端或客户端构造失败时返回错误
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()?;
        Ok(Self { client })
    }

    async fn fetch_within(
        &self,
        url: &str,
        policy: &SubscriptionFetchPolicy,
    ) -> Result<FetchedDocument, SubscriptionFetchError> {
        let mut target =
            reqwest::Url::parse(url).map_err(|_| SubscriptionFetchError::TargetRejected)?;
        let mut redirects = 0usize;
        loop {
            // 首跳与每一跳跳转目标都按同一策略复核，并在提问前丢弃地址内的凭据
            target
                .set_username("")
                .map_err(|()| SubscriptionFetchError::TargetRejected)?;
            target
                .set_password(None)
                .map_err(|()| SubscriptionFetchError::TargetRejected)?;
            let host = target
                .host_str()
                .ok_or(SubscriptionFetchError::TargetRejected)?
                .to_owned();
            policy
                .allow(target.scheme(), &host)
                .map_err(|_| SubscriptionFetchError::TargetRejected)?;
            let mut response = self
                .client
                .get(target.clone())
                .header(ACCEPT, "*/*")
                .header(ACCEPT_ENCODING, "gzip, deflate")
                .send()
                .await
                .map_err(|_| SubscriptionFetchError::Failed)?;
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
        // 时限覆盖连接、响应头、正文与全部跳转，调用方传入的是整次预览的剩余预算
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
