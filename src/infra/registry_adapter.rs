use crate::app::ports::RegistryPort;
use async_trait::async_trait;

pub struct JsonRegistry;

#[async_trait]
impl RegistryPort for JsonRegistry {
    async fn load_parse_plan(&self, source_id: &str) -> Result<String, String> {
        // Load the registry JSON and return parse_plan_ref or default
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let reg_path = base.join("registry/sources").join(format!("{}.json", source_id));
        let spec = crate::pipeline::ingestion::registry::load_source_spec(&reg_path)
            .map_err(|e| format!("load_source_spec_failed: {}", e))?;
        Ok(spec.parse_plan_ref.unwrap_or_else(|| "parse_plan:wix_calendar_v1".to_string()))
    }
}

