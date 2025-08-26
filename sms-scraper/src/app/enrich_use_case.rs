use anyhow::Result;
use crate::app::ports::EnrichOutputPort;
use crate::pipeline::processing::enrich::{
    Enricher, EnrichedRecord, DefaultEnricher, MetricsEnricher
};
use crate::pipeline::processing::quality_gate::QualityAssessedRecord;

/// Use case for enriching quality-assessed records with contextual metadata
pub struct EnrichUseCase {
    enricher: Box<dyn Enricher + Send + Sync>,
    output: Box<dyn EnrichOutputPort>,
}

impl EnrichUseCase {
    pub fn new(
        enricher: Box<dyn Enricher + Send + Sync>,
        output: Box<dyn EnrichOutputPort>,
    ) -> Self {
        Self {
            enricher,
            output,
        }
    }

    /// Create a use case with the default enricher
    pub fn with_default_enricher(output: Box<dyn EnrichOutputPort>) -> Self {
        Self {
            enricher: Box::new(MetricsEnricher::new(DefaultEnricher::new())),
            output,
        }
    }

    /// Enrich a single quality-assessed record
    pub async fn enrich_record(&self, record: &QualityAssessedRecord) -> Result<EnrichedRecord> {
        // Apply enrichment logic (metrics are handled by MetricsEnricher wrapper)
        let enriched_record = self.enricher.enrich(record)?;

        // Write enriched record to output
        self.output.write_enriched_record(&enriched_record).await?;

        Ok(enriched_record)
    }

    /// Enrich multiple quality-assessed records in batch
    pub async fn enrich_batch(&self, records: &[QualityAssessedRecord]) -> Result<Vec<EnrichedRecord>> {
        let mut all_enriched = Vec::new();

        for record in records {
            let enriched = self.enrich_record(record).await?;
            all_enriched.push(enriched);
        }

        crate::observability::metrics::enrich::batch_processed(records.len());
        Ok(all_enriched)
    }

    /// Get statistics for the current batch enrichment
    pub fn get_batch_stats(enriched_records: &[EnrichedRecord]) -> EnrichBatchStats {
        let mut stats = EnrichBatchStats::default();
        
        for record in enriched_records {
            stats.total_records += 1;
            
            // Track confidence stats
            let confidence = record.enrichment.confidence;
            if stats.min_confidence.is_none() || confidence < stats.min_confidence.unwrap() {
                stats.min_confidence = Some(confidence);
            }
            if stats.max_confidence.is_none() || confidence > stats.max_confidence.unwrap() {
                stats.max_confidence = Some(confidence);
            }
            stats.avg_confidence = if stats.total_records == 1 {
                confidence
            } else {
                (stats.avg_confidence * (stats.total_records - 1) as f64 + confidence) / stats.total_records as f64
            };
            
            // Count records with spatial binning
            if record.enrichment.spatial_bin.is_some() {
                stats.spatially_binned += 1;
            }
            
            // Count records with city identification
            if record.enrichment.city.is_some() {
                stats.city_identified += 1;
            }
            
            // Count records with coordinates
            if record.enrichment.geo_properties.distance_from_center_km.is_some() {
                stats.with_coordinates += 1;
            }
            
            // Count total tags
            stats.total_tags += record.enrichment.tags.len();
            
            // Count warnings
            stats.total_warnings += record.enrichment.warnings.len();
        }
        
        // Calculate average tags per record
        stats.avg_tags_per_record = if stats.total_records > 0 {
            stats.total_tags as f64 / stats.total_records as f64
        } else {
            0.0
        };
        
        stats
    }
}

/// Statistics for a batch of enrichment operations
#[derive(Debug, Default)]
pub struct EnrichBatchStats {
    pub total_records: usize,
    pub spatially_binned: usize,
    pub city_identified: usize,
    pub with_coordinates: usize,
    pub total_tags: usize,
    pub total_warnings: usize,
    pub min_confidence: Option<f64>,
    pub max_confidence: Option<f64>,
    pub avg_confidence: f64,
    pub avg_tags_per_record: f64,
}

impl EnrichBatchStats {
    /// Calculate spatial binning rate as percentage
    pub fn spatial_binning_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        self.spatially_binned as f64 / self.total_records as f64 * 100.0
    }

    /// Calculate city identification rate as percentage
    pub fn city_identification_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        self.city_identified as f64 / self.total_records as f64 * 100.0
    }
    
    /// Calculate coordinate availability rate as percentage
    pub fn coordinate_availability_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        self.with_coordinates as f64 / self.total_records as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::EnrichOutputPort;
    use crate::pipeline::processing::enrich::EnrichedRecord;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockEnrichOutput {
        pub records: Arc<Mutex<Vec<EnrichedRecord>>>,
    }

    impl MockEnrichOutput {
        pub fn new() -> Self {
            Self {
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl EnrichOutputPort for MockEnrichOutput {
        async fn write_enriched_record(&self, record: &EnrichedRecord) -> Result<()> {
            self.records.lock().await.push(record.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_enrich_use_case() {
        let output = Box::new(MockEnrichOutput::new());
        let records_ref = output.records.clone();
        let use_case = EnrichUseCase::with_default_enricher(output);

        // Create a test quality-assessed record
        use crate::domain::Venue;
        use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
        use crate::pipeline::processing::quality_gate::{QualityAssessedRecord, QualityAssessment, QualityDecision};
        use chrono::Utc;

        let venue = Venue {
            id: None,
            name: "Test Venue".to_string(),
            name_lower: "test venue".to_string(),
            slug: "test-venue".to_string(),
            latitude: 47.6131, // Belltown area
            longitude: -122.3424,
            address: "123 Test St".to_string(),
            postal_code: "98101".to_string(),
            city: "Seattle".to_string(),
            venue_url: None,
            venue_image_url: None,
            description: None,
            neighborhood: None,
            show_venue: true,
            created_at: Utc::now(),
        };

        let quality_assessed_record = QualityAssessedRecord {
            normalized_record: NormalizedRecord {
                entity: NormalizedEntity::Venue(venue),
                provenance: RecordProvenance {
                    envelope_id: "test_envelope".to_string(),
                    source_id: "test_source".to_string(),
                    payload_ref: "test_payload".to_string(),
                    record_path: "$.venues[0]".to_string(),
                    normalized_at: Utc::now(),
                },
                normalization: NormalizationMetadata {
                    confidence: 0.8,
                    warnings: Vec::new(),
                    geocoded: false,
                    strategy: "default".to_string(),
                },
            },
            quality_assessment: QualityAssessment {
                decision: QualityDecision::Accept,
                quality_score: 0.85,
                issues: Vec::new(),
                rule_version: "v1.0.0".to_string(),
            },
            assessed_at: Utc::now(),
        };

        let result = use_case.enrich_record(&quality_assessed_record).await;
        assert!(result.is_ok());

        let written_records = records_ref.lock().await;
        assert_eq!(written_records.len(), 1);
        
        let enriched = &written_records[0];
        assert!(enriched.enrichment.confidence > 0.7);
        assert_eq!(enriched.enrichment.city, Some("Seattle".to_string()));
        assert!(enriched.enrichment.spatial_bin.is_some());
    }
}
