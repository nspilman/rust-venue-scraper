use crate::app::ports::PayloadStorePort;
use async_trait::async_trait;

pub struct CasPayloadStore;

#[async_trait]
impl PayloadStorePort for CasPayloadStore {
    async fn get(&self, payload_ref: &str) -> Result<Vec<u8>, String> {
        // payload_ref format: cas:sha256:<hex>
        let prefix = "cas:sha256:";
        let hex = payload_ref.strip_prefix(prefix).ok_or_else(|| "bad_payload_ref".to_string())?;
        // Try Supabase first if env vars present, else local filesystem via ingest_log_reader resolution
        if (std::env::var("SUPABASE_URL").is_ok() || std::env::var("SUPABASE_PROJECT_REF").is_ok())
            && std::env::var("SUPABASE_BUCKET").is_ok()
        {
            let project_ref = std::env::var("SUPABASE_PROJECT_REF").ok();
            let supabase_url = std::env::var("SUPABASE_URL")
                .ok()
                .or_else(|| project_ref.map(|r| format!("https://{}.supabase.co", r)))
                .ok_or_else(|| "missing_supabase_url".to_string())?;
            let bucket = std::env::var("SUPABASE_BUCKET").map_err(|_| "missing_supabase_bucket".to_string())?;
            let prefix = std::env::var("SUPABASE_PREFIX").unwrap_or_default();
            let key = if prefix.is_empty() {
                format!("sha256/{}/{}/{}", &hex[0..2], &hex[2..4], hex)
            } else {
                format!("{}/sha256/{}/{}/{}", prefix.trim_end_matches('/'), &hex[0..2], &hex[2..4], hex)
            };
            let base = supabase_url.trim_end_matches('/').to_string();
            let client = reqwest::Client::new();
            let public_url = format!("{}/storage/v1/object/public/{}/{}", base, bucket, key);
            let mut resp = client.get(public_url).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                if let Ok(key_hdr) = std::env::var("SUPABASE_SERVICE_ROLE_KEY").or_else(|_| std::env::var("SUPABASE_ANON_KEY")) {
                    let auth_url = format!("{}/storage/v1/object/{}/{}", base, bucket, key);
                    resp = client
                        .get(auth_url)
                        .header("Authorization", format!("Bearer {}", key_hdr))
                        .header("apikey", key_hdr.clone())
                        .send()
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
            if !resp.status().is_success() {
                return Err(format!("supabase_fetch_failed: {}", resp.status()));
            }
            let b = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();
            return Ok(b);
        }
        // Local path: resolve via repo's ingest_log_reader helper
        let reader = crate::ingest_log_reader::IngestLogReader::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"));
        if let Some(path) = reader.resolve_payload_path(payload_ref) {
            return std::fs::read(path).map_err(|e| e.to_string());
        }
        Err("payload_path_not_found".to_string())
    }
}

