use crate::app::ports::CadencePort;
use async_trait::async_trait;

pub struct IngestMetaCadence;

#[async_trait]
impl CadencePort for IngestMetaCadence {
    async fn should_run(&self, source_id: &str, min_interval_secs: i64) -> Result<bool, String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let bypass = std::env::var("SMS_BYPASS_CADENCE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
        if bypass { return Ok(true); }
        let meta = crate::pipeline::ingestion::ingest_meta::IngestMeta::open_at_root(&root).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        if let Some(last) = meta.get_last_fetched_at(source_id).map_err(|e| e.to_string())? {
            Ok(now - last >= min_interval_secs)
        } else {
            Ok(true)
        }
    }
    async fn mark_run(&self, source_id: &str) -> Result<(), String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
        let meta = crate::pipeline::ingestion::ingest_meta::IngestMeta::open_at_root(&root).map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();
        meta.set_last_fetched_at(source_id, now).map_err(|e| e.to_string())?;
        Ok(())
    }
}

