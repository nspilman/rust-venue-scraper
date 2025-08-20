use std::collections::HashMap;
use anyhow::Result;

use super::normalizers::{SourceNormalizer, MetricsNormalizer, SeaMonsterNormalizer, DarrellsTavernNormalizer, BlueMoonNormalizer, KexpNormalizer};
use crate::observability::metrics;
use super::NormalizedRecord;
use crate::pipeline::processing::parser::ParsedRecord;

/// Registry for source-specific normalization strategies
pub struct NormalizationRegistry {
    normalizers: HashMap<String, Box<dyn SourceNormalizer>>,
}

impl NormalizationRegistry {
    /// Create a new normalization registry with predefined normalizers
    pub fn new() -> Self {
        let mut normalizers: HashMap<String, Box<dyn SourceNormalizer>> = HashMap::new();
        
        // Register all built-in source-specific normalizers with metrics
        normalizers.insert("sea_monster".to_string(), 
            Box::new(MetricsNormalizer::new(SeaMonsterNormalizer::new())));
        normalizers.insert("darrells_tavern".to_string(), 
            Box::new(MetricsNormalizer::new(DarrellsTavernNormalizer::new())));
        normalizers.insert("blue_moon".to_string(), 
            Box::new(MetricsNormalizer::new(BlueMoonNormalizer::new())));
        normalizers.insert("kexp".to_string(), 
            Box::new(MetricsNormalizer::new(KexpNormalizer::new())));
        
        Self {
            normalizers,
        }
    }

    /// Test-only: list registered source IDs
    #[cfg(test)]
    pub fn list_sources(&self) -> Vec<&str> {
        self.normalizers.keys().map(|k| k.as_str()).collect()
    }

    /// Get the appropriate normalizer for a source
    pub fn get_normalizer(&self, source_id: &str) -> Option<&dyn SourceNormalizer> {
        self.normalizers
            .get(source_id)
            .map(|n| n.as_ref())
    }

    /// Normalize a record using the appropriate source-specific normalizer
    pub fn normalize(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        // Record batch processing metrics
        metrics::normalize::batch_processed(1);
        
        if let Some(normalizer) = self.get_normalizer(&record.source_id) {
            normalizer.normalize(record)
        } else {
            metrics::normalize::warning_logged(&format!("no_normalizer_for_source_{}", record.source_id));
            Err(anyhow::anyhow!("No normalizer registered for source: {}", record.source_id))
        }
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_registry_has_built_in_normalizers() {
        let registry = NormalizationRegistry::new();
        
        let sources = registry.list_sources();
        assert!(sources.contains(&"sea_monster"));
        assert!(sources.contains(&"darrells_tavern"));
        assert!(sources.contains(&"blue_moon"));
        assert!(sources.contains(&"kexp"));
    }

    #[test]
    fn test_registry_returns_error_for_unknown_source() {
        let registry = NormalizationRegistry::new();

        let record = ParsedRecord {
            source_id: "unknown_source".to_string(),
            envelope_id: "test".to_string(),
            payload_ref: "test".to_string(),
            record_path: "test".to_string(),
            record: json!({
                "title": "Test Event",
                "venue": "Test Venue"
            }),
        };

        // Should return an error for unknown sources
        let result = registry.normalize(&record);
        assert!(result.is_err());
    }
}
