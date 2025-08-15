use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChecksumMeta {
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PayloadMeta {
    pub mime_type: String,
    pub size_bytes: u64,
    pub checksum: ChecksumMeta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestMeta {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimingMeta {
    pub fetched_at: DateTime<Utc>,
    pub gateway_received_at: Option<DateTime<Utc>>, // set by gateway
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LegalMeta {
    pub license_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvelopeSubmissionV1 {
    pub envelope_version: String, // "1.0.0"
    pub source_id: String,
    pub idempotency_key: String,
    pub payload_meta: PayloadMeta,
    pub request: RequestMeta,
    pub timing: TimingMeta,
    pub legal: LegalMeta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StampedEnvelopeV1 {
    pub envelope_version: String,
    pub envelope_id: String,
    pub accepted_at: DateTime<Utc>,
    pub payload_ref: String,
    pub dedupe_of: Option<String>,
    pub envelope: EnvelopeSubmissionV1,
}
