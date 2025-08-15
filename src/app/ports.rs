use async_trait::async_trait;

#[async_trait]
pub trait PayloadStorePort: Send + Sync {
    async fn get(&self, payload_ref: &str) -> Result<Vec<u8>, String>;
}

#[async_trait]
pub trait RegistryPort: Send + Sync {
    async fn load_parse_plan(&self, source_id: &str) -> Result<String, String>;
}

#[async_trait]
pub trait ParserPort: Send + Sync {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String>;
}

pub trait ParserFactory: Send + Sync {
    fn for_plan(&self, plan: &str) -> Option<Box<dyn ParserPort>>;
}

// Ingest-side ports
#[async_trait]
pub trait HttpClientPort: Send + Sync {
    async fn get(&self, url: &str) -> Result<HttpGetResult, String>;
}

#[derive(Clone, Debug)]
pub struct HttpGetResult {
    pub status: u16,
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub content_length: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[async_trait]
pub trait RateLimiterPort: Send + Sync {
    async fn acquire(&self, bytes: u64);
}

#[async_trait]
pub trait CadencePort: Send + Sync {
    async fn should_run(&self, source_id: &str, min_interval_secs: i64) -> Result<bool, String>;
    async fn mark_run(&self, source_id: &str) -> Result<(), String>;
}

#[async_trait]
pub trait GatewayPort: Send + Sync {
    async fn accept(&self, env: crate::pipeline::ingestion::envelope::EnvelopeSubmissionV1, bytes: Vec<u8>) -> Result<crate::pipeline::ingestion::envelope::StampedEnvelopeV1, String>;
}

