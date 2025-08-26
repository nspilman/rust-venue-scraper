use anyhow::Result;

use crate::app::ports::NormalizeOutputPort;
use crate::pipeline::processing::normalize::{NormalizedRecord, NormalizationRegistry};
use crate::pipeline::processing::parser::ParsedRecord;

/// Use case for normalizing parsed records into canonical domain entities
pub struct NormalizeUseCase {
    registry: NormalizationRegistry,
    output: Box<dyn NormalizeOutputPort>,
}

impl NormalizeUseCase {
    pub fn new(
        output: Box<dyn NormalizeOutputPort>,
    ) -> Self {
        Self {
            registry: NormalizationRegistry::new(),
            output,
        }
    }

    /// Normalize a single parsed record
    pub async fn normalize_record(&self, record: &ParsedRecord) -> Result<Vec<NormalizedRecord>> {
        // Apply normalization logic
        let normalized_records = self.registry.normalize(record)?;

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
        let use_case = NormalizeUseCase::new(output);

        // This test might fail because the new registry requires specific source IDs
        // We'll use "sea_monster" since it's one of the registered normalizers
        let parsed_record = ParsedRecord {
            source_id: "sea_monster".to_string(),
            envelope_id: "test_envelope".to_string(),
            payload_ref: "test_payload".to_string(),
            record_path: "$.events[0]".to_string(),
            record: json!({
                "title": "Test Event",
                "scheduling": {
                    "startDateFormatted": "January 15, 2025"
                },
                "location": {
                    "name": "Sea Monster Lounge"
                }
            }),
        };

        let result = use_case.normalize_record(&parsed_record).await;
        assert!(result.is_ok());

        let written_records = records_ref.lock().await;
        // For sea_monster source, we expect Event, Venue, and Artist records
        assert!(written_records.len() >= 2); // At least Event and Venue
    }
}
