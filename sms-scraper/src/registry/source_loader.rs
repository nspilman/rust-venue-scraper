use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use sms_core::common::error::{Result, ScraperError};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SourceEndpoint {
    pub url: String,
    pub method: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SourceConfig {
    pub source_id: String,
    pub enabled: bool,
    pub endpoints: Vec<SourceEndpoint>,
    // Existing field for backward compatibility
    pub parse_plan_ref: Option<String>,
    // New pipeline configuration
    pub pipeline: Option<PipelineConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PipelineConfig {
    pub parser_id: String,
    pub normalizer_id: String,
    pub content_type: String,
    pub parser_type: String,
}

#[derive(Clone)]
pub struct SourceRegistry {
    sources: HashMap<String, SourceConfig>,
}

impl SourceRegistry {
    /// Load all source configurations from the registry directory
    pub fn load_from_directory<P: AsRef<Path>>(registry_dir: P) -> Result<Self> {
        let mut sources = HashMap::new();
        
        let dir_path = registry_dir.as_ref();
        if !dir_path.exists() {
            return Err(ScraperError::Api {
                message: format!("Registry directory does not exist: {}", dir_path.display()),
            });
        }

        let entries = fs::read_dir(dir_path).map_err(|e| ScraperError::Api {
            message: format!("Failed to read registry directory: {}", e),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ScraperError::Api {
                message: format!("Failed to read directory entry: {}", e),
            })?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| ScraperError::Api {
                    message: format!("Failed to read source file {}: {}", path.display(), e),
                })?;
                
                let config: SourceConfig = serde_json::from_str(&content).map_err(|e| ScraperError::Api {
                    message: format!("Failed to parse source config {}: {}", path.display(), e),
                })?;
                
                sources.insert(config.source_id.clone(), config);
            }
        }

        Ok(Self { sources })
    }

    /// Get the primary URL for a source
    pub fn get_source_url(&self, source_id: &str) -> Result<String> {
        let source = self.sources.get(source_id).ok_or_else(|| ScraperError::Api {
            message: format!("Source not found in registry: {}", source_id),
        })?;

        if !source.enabled {
            return Err(ScraperError::Api {
                message: format!("Source is disabled: {}", source_id),
            });
        }

        let endpoint = source.endpoints.first().ok_or_else(|| ScraperError::Api {
            message: format!("No endpoints found for source: {}", source_id),
        })?;

        Ok(endpoint.url.clone())
    }

    /// Check if a source is enabled
    pub fn is_source_enabled(&self, source_id: &str) -> bool {
        self.sources.get(source_id).map_or(false, |s| s.enabled)
    }

    /// Get all enabled source IDs
    pub fn get_enabled_sources(&self) -> Vec<String> {
        self.sources
            .values()
            .filter(|s| s.enabled)
            .map(|s| s.source_id.clone())
            .collect()
    }
    
    /// Get source configuration by ID
    pub fn get_source_config(&self, source_id: &str) -> Option<&SourceConfig> {
        self.sources.get(source_id)
    }
}
