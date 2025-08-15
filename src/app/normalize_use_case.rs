use anyhow::Result;

use crate::app::ports::NormalizeOutputPort;
use crate::pipeline::processing::normalize::{DefaultNormalizer, NormalizedRecord, Normalizer};
use crate::pipeline::processing::parser::ParsedRecord;

/// Use case for normalizing parsed records into canonical domain entities
pub struct NormalizeUseCase {
    normalizer: Box<dyn Normalizer + Send + Sync>,
    output: Box<dyn NormalizeOutputPort>,
}

impl NormalizeUseCase {
    pub fn new(
        normalizer: Box<dyn Normalizer + Send + Sync>,
        output: Box<dyn NormalizeOutputPort>,
    ) -> Self {
        Self {
            normalizer,
            output,
        }
    }

    /// Create a use case with the default normalizer
    pub fn with_default_normalizer(output: Box<dyn NormalizeOutputPort>) -> Self {
        Self {
            normalizer: Box::new(DefaultNormalizer { geocoder: None }),
            output,
        }
    }

    /// Normalize a single parsed record
    pub async fn normalize_record(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        // Apply normalization logic
        let normalized_records = self.normalizer.normalize(record)?;

        // Emit metrics for normalization
        for record in &normalized_records {
            crate::observability::metrics::normalize::record_normalized(&record.normalization.strategy);
            crate::observability::metrics::normalize::confidence_recorded(record.normalization.confidence);
            
            if record.normalization.geocoded {
                crate::observability::metrics::normalize::geocoding_performed();
            }
            
            for warning in &record.normalization.warnings {
                crate::observability::metrics::normalize::warning_logged(warning);
            }
        }

        // Write normalized records to output
        for record in &normalized_records {
            self.output.write_normalized_record(record).await?;
        }

        Ok(normalized_records)
    }

    /// Normalize multiple parsed records in batch
    pub async fn normalize_batch(&self, records: &[ParsedRecord]) -> Result<Vec<NormalizedRecord>> {
        let mut all_normalized = Vec::new();

        for record in records {
            let normalized = self.normalize_record(record).await?;
            all_normalized.extend(normalized);
        }

        crate::observability::metrics::normalize::batch_processed(records.len());
        Ok(all_normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::NormalizeOutputPort;
    use crate::pipeline::processing::normalize::NormalizedRecord;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    struct MockNormalizeOutput {
        pub records: Arc<tokio::sync::Mutex<Vec<NormalizedRecord>>>,
    }

    impl MockNormalizeOutput {
        pub fn new() -> Self {
            Self {
                records: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl NormalizeOutputPort for MockNormalizeOutput {
        async fn write_normalized_record(&self, record: &NormalizedRecord) -> Result<()> {
            self.records.lock().await.push(record.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_normalize_use_case() {
        let output = Box::new(MockNormalizeOutput::new());
        let records_ref = output.records.clone();
        let use_case = NormalizeUseCase::with_default_normalizer(output);

        let parsed_record = ParsedRecord {
            source_id: "test_source".to_string(),
            envelope_id: "test_envelope".to_string(),
            payload_ref: "test_payload".to_string(),
            record_path: "$.events[0]".to_string(),
            record: json!({
                "title": "Test Event",
                "event_day": "2025-08-15",
                "venue": "Test Venue",
                "artist": "Test Artist"
            }),
        };

        let result = use_case.normalize_record(&parsed_record).await;
        assert!(result.is_ok());

        let written_records = records_ref.lock().await;
        assert_eq!(written_records.len(), 3); // Event, Venue, Artist
    }
}
