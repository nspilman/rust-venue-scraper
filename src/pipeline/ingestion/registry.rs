use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointSpec {
    pub url: String,
    pub method: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentSpec {
    pub allowed_mime_types: Vec<String>,
    pub max_payload_size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PolicySpec {
    pub license_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RateLimitsSpec {
    pub requests_per_min: Option<u64>,
    pub bytes_per_min: Option<u64>,
    pub concurrency: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceSpecV1 {
    pub source_id: String,
    pub enabled: bool,
    pub endpoints: Vec<EndpointSpec>,
    pub content: ContentSpec,
    pub policy: PolicySpec,
    #[serde(default)]
    pub parse_plan_ref: Option<String>,
    #[serde(default)]
    pub rate_limits: RateLimitsSpec,
}

pub fn load_source_spec(path: &Path) -> anyhow::Result<SourceSpecV1> {
    let raw = fs::read_to_string(path)?;
    let spec: SourceSpecV1 = serde_json::from_str(&raw)?;
    Ok(spec)
}
