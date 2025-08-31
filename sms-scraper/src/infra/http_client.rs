use crate::app::ports::{HttpClientPort, HttpGetResult};
use async_trait::async_trait;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};

pub struct ReqwestHttp;

#[async_trait]
impl HttpClientPort for ReqwestHttp {
    async fn get(&self, url: &str) -> Result<HttpGetResult, String> {
        let client = reqwest::Client::new();
        tracing::info!("HTTP GET request to: {}", url);
        let resp = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let headers = resp.headers().clone();
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();
        tracing::info!("HTTP response: status={}, size={} bytes, contains wix-warmup-data: {}", 
            status, bytes.len(), 
            String::from_utf8_lossy(&bytes).contains("wix-warmup-data"));
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let content_length: u64 = headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(bytes.len() as u64);
        let etag = headers.get(ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
        let last_modified = headers
            .get(LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Ok(HttpGetResult { status, bytes, content_type, content_length, etag, last_modified })
    }
}

