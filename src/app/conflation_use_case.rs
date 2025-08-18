use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

use crate::app::ports::ConflationOutputPort;
use crate::observability::metrics::{emit_counter, emit_histogram, emit_gauge, MetricName};
use crate::pipeline::processing::conflation::{ConflatedRecord, Conflator, DefaultConflator, ResolutionDecision};
use crate::pipeline::processing::enrich::EnrichedRecord;

/// Use case for performing entity conflation on enriched records
pub struct ConflationUseCase {
    /// The conflator implementation for entity resolution
    conflator: Box<dyn Conflator + Send + Sync>,
    /// Output port for writing conflated records
    output_port: Arc<dyn ConflationOutputPort>,
}

impl ConflationUseCase {
    /// Create a new conflation use case with default conflator
    pub fn new(output_port: Arc<dyn ConflationOutputPort>) -> Self {
        Self {
            conflator: Box::new(DefaultConflator::new()),
            output_port,
        }
    }

    /// Create a conflation use case with a custom conflator
    pub fn with_conflator(
        conflator: Box<dyn Conflator + Send + Sync>,
        output_port: Arc<dyn ConflationOutputPort>,
    ) -> Self {
        Self {
            conflator,
            output_port,
        }
    }

    /// Process a single enriched record through conflation
    pub async fn conflate_record(&self, record: &EnrichedRecord) -> Result<ConflatedRecord> {
        let start_time = std::time::Instant::now();
        
        info!(
            "Starting conflation for record from source: {}",
            record.quality_assessed_record.normalized_record.provenance.source_id
        );

        // Emit input metrics
        emit_counter(MetricName::ConflationRecordsProcessed, 1.0);

        // Determine entity type for metrics
        let entity_type = self.determine_entity_type_string(record);
        
        // Perform conflation
        let conflated_record = match self.conflator.conflate(record) {
            Ok(record) => {
                debug!(
                    "Conflation successful for {} with decision: {:?}",
                    entity_type,
                    record.conflation.resolution_decision
                );
                record
            }
            Err(e) => {
                error!("Conflation failed: {}", e);
                emit_counter(MetricName::ConflationRecordsFailed, 1.0);
                return Err(e);
            }
        };

        // Emit resolution decision metrics
        self.emit_resolution_metrics(&conflated_record, &entity_type);

        // Emit confidence metrics
        emit_histogram(
            MetricName::ConflationConfidenceScore,
            conflated_record.conflation.confidence,
        );

        // Emit warnings if present
        if !conflated_record.conflation.warnings.is_empty() {
            emit_counter(MetricName::ConflationWarnings, conflated_record.conflation.warnings.len() as f64);
            for warning in &conflated_record.conflation.warnings {
                warn!("Conflation warning: {}", warning);
            }
        }

        // Emit deduplication metrics
        if conflated_record.conflation.deduplication.is_potential_duplicate {
            emit_counter(MetricName::ConflationPotentialDuplicates, 1.0);
            info!(
                "Potential duplicates detected for entity: {} duplicates found",
                conflated_record.conflation.deduplication.potential_duplicates.len()
            );
        }

        // Emit alternative matches metrics
        if !conflated_record.conflation.alternatives.is_empty() {
            emit_counter(
                MetricName::ConflationAlternativeMatches,
                conflated_record.conflation.alternatives.len() as f64,
            );
            debug!(
                "Alternative matches considered: {}",
                conflated_record.conflation.alternatives.len()
            );
        }

        // Write conflated record to output
        if let Err(e) = self.output_port.write_conflated_record(&conflated_record).await {
            error!("Failed to write conflated record: {}", e);
            emit_counter(MetricName::ConflationOutputFailed, 1.0);
            return Err(e);
        }

        // Emit timing metrics
        let processing_duration = start_time.elapsed();
        emit_histogram(
            MetricName::ConflationProcessingDuration,
            processing_duration.as_secs_f64(),
        );

        emit_counter(MetricName::ConflationRecordsSuccessful, 1.0);
        info!(
            "Conflation completed for {} in {:.2}ms with entity ID: {}",
            entity_type,
            processing_duration.as_millis(),
            conflated_record.canonical_entity_id.id
        );

        Ok(conflated_record)
    }

    /// Process a batch of enriched records through conflation
    pub async fn conflate_batch(&self, records: &[EnrichedRecord]) -> Result<Vec<ConflatedRecord>> {
        let start_time = std::time::Instant::now();
        let batch_size = records.len();

        info!("Starting conflation batch processing for {} records", batch_size);
        emit_counter(MetricName::ConflationBatchesProcessed, 1.0);
        emit_gauge(MetricName::ConflationBatchSize, batch_size as f64);

        let mut conflated_records = Vec::with_capacity(batch_size);
        let mut successful_count = 0;
        let mut failed_count = 0;

        for (index, record) in records.iter().enumerate() {
            debug!("Processing record {} of {} in batch", index + 1, batch_size);

            match self.conflate_record(record).await {
                Ok(conflated_record) => {
                    conflated_records.push(conflated_record);
                    successful_count += 1;
                }
                Err(e) => {
                    error!("Failed to conflate record {} in batch: {}", index + 1, e);
                    failed_count += 1;
                    // Continue processing other records in the batch
                }
            }
        }

        let batch_duration = start_time.elapsed();
        emit_histogram(
            MetricName::ConflationBatchProcessingDuration,
            batch_duration.as_secs_f64(),
        );

        emit_counter(MetricName::ConflationBatchRecordsSuccessful, successful_count as f64);
        if failed_count > 0 {
            emit_counter(MetricName::ConflationBatchRecordsFailed, failed_count as f64);
        }

        info!(
            "Conflation batch completed: {}/{} successful in {:.2}ms",
            successful_count,
            batch_size,
            batch_duration.as_millis()
        );

        if failed_count > 0 {
            warn!(
                "Conflation batch had {} failures out of {} records",
                failed_count, batch_size
            );
        }

        // Emit batch success metrics
        emit_counter(MetricName::ConflationBatchesSuccessful, 1.0);

        Ok(conflated_records)
    }

    /// Get conflation statistics for monitoring
    pub fn get_conflation_stats(&self) -> ConflationStats {
        // In a production implementation, these would be retrieved from the conflator
        // or a statistics service. For now, return placeholder values.
        ConflationStats {
            total_entities: 0,
            new_entities_created: 0,
            existing_entities_matched: 0,
            potential_duplicates: 0,
            uncertain_resolutions: 0,
            average_confidence: 0.0,
        }
    }

    /// Emit metrics based on the resolution decision
    fn emit_resolution_metrics(&self, conflated_record: &ConflatedRecord, entity_type: &str) {
        match &conflated_record.conflation.resolution_decision {
            ResolutionDecision::NewEntity => {
                emit_counter(MetricName::ConflationNewEntities, 1.0);
                info!("New {} entity created: {}", entity_type, conflated_record.canonical_entity_id.id);
            }
            ResolutionDecision::MatchedExisting(entity_id) => {
                emit_counter(MetricName::ConflationMatchedExisting, 1.0);
                info!("Matched existing {} entity: {}", entity_type, entity_id.id);
            }
            ResolutionDecision::UpdatedExisting(entity_id) => {
                emit_counter(MetricName::ConflationUpdatedExisting, 1.0);
                info!("Updated existing {} entity: {}", entity_type, entity_id.id);
            }
            ResolutionDecision::Duplicate(entity_id) => {
                emit_counter(MetricName::ConflationDuplicates, 1.0);
                info!("Duplicate {} detected for entity: {}", entity_type, entity_id.id);
            }
            ResolutionDecision::Uncertain => {
                emit_counter(MetricName::ConflationUncertainResolutions, 1.0);
                warn!("Uncertain conflation resolution for {}", entity_type);
            }
        }
    }

    /// Determine entity type as string for logging and metrics
    fn determine_entity_type_string(&self, record: &EnrichedRecord) -> String {
        use crate::pipeline::processing::normalize::NormalizedEntity;

        match &record.quality_assessed_record.normalized_record.entity {
            NormalizedEntity::Venue(_) => "venue".to_string(),
            NormalizedEntity::Event(_) => "event".to_string(),
            NormalizedEntity::Artist(_) => "artist".to_string(),
        }
    }
}

/// Statistics about conflation operations
#[derive(Debug, Clone)]
pub struct ConflationStats {
    /// Total number of canonical entities known
    pub total_entities: u64,
    /// Number of new entities created
    pub new_entities_created: u64,
    /// Number of records matched to existing entities
    pub existing_entities_matched: u64,
    /// Number of potential duplicates identified
    pub potential_duplicates: u64,
    /// Number of uncertain resolutions requiring manual review
    pub uncertain_resolutions: u64,
    /// Average confidence score across all conflations
    pub average_confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ConflationOutputPort;
    use crate::domain::{Venue};
    use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
    use crate::pipeline::processing::quality_gate::{QualityAssessment, QualityDecision, QualityAssessedRecord};
    use crate::pipeline::processing::enrich::{EnrichedRecord, EnrichmentMetadata, GeoProperties, PopulationDensity, ReferenceVersions};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    // Mock output port for testing
    struct MockConflationOutputPort {
        written_records: Arc<Mutex<Vec<ConflatedRecord>>>,
    }

    impl MockConflationOutputPort {
        fn new() -> Self {
            Self {
                written_records: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_written_records(&self) -> Vec<ConflatedRecord> {
            self.written_records.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl ConflationOutputPort for MockConflationOutputPort {
        async fn write_conflated_record(&self, record: &ConflatedRecord) -> Result<()> {
            self.written_records.lock().unwrap().push(record.clone());
            Ok(())
        }
    }

    fn create_test_enriched_record() -> EnrichedRecord {
        let venue = Venue {
            id: None,
            name: "Test Venue".to_string(),
            name_lower: "test venue".to_string(),
            slug: "test-venue".to_string(),
            latitude: 47.6131,
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
            updated_at: Utc::now(),
        };

        let normalized_record = NormalizedRecord {
            id: Uuid::new_v4(),
            entity: NormalizedEntity::Venue(venue),
            normalized_at: Utc::now(),
            normalization: NormalizationMetadata {
                strategy: "test".to_string(),
                confidence: 1.0,
                warnings: Vec::new(),
                transformations: Vec::new(),
            },
            provenance: RecordProvenance {
                envelope_id: Uuid::new_v4(),
                source_id: "test_source".to_string(),
                source_record_id: "test_record".to_string(),
                ingested_at: Utc::now(),
            },
        };

        let quality_assessment = QualityAssessment {
            quality_score: 0.95,
            decision: QualityDecision::Accept,
            checks_passed: 5,
            checks_failed: 0,
            quality_issues: Vec::new(),
            assessed_at: Utc::now(),
        };

        let quality_assessed_record = QualityAssessedRecord {
            normalized_record,
            quality_assessment,
        };

        let enrichment = EnrichmentMetadata {
            city: Some("Seattle".to_string()),
            district: Some("Belltown".to_string()),
            region: Some("King County, WA".to_string()),
            spatial_bin: Some("seattle_grid_47_-122".to_string()),
            tags: vec!["venue".to_string(), "test".to_string()],
            geo_properties: GeoProperties {
                within_city_bounds: true,
                distance_from_center_km: Some(5.0),
                population_density: PopulationDensity::Urban,
                transit_accessibility: Some(0.8),
                nearby_landmarks: Vec::new(),
            },
            reference_versions: ReferenceVersions {
                city_boundaries: Some("test_v1".to_string()),
                admin_boundaries: Some("test_v1".to_string()),
                spatial_grid: Some("test_v1".to_string()),
                poi_data: Some("test_v1".to_string()),
            },
            strategy: "test_enrichment".to_string(),
            confidence: 0.9,
            warnings: Vec::new(),
        };

        EnrichedRecord {
            quality_assessed_record,
            enrichment,
            enriched_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_conflate_single_record() {
        let mock_output = Arc::new(MockConflationOutputPort::new());
        let use_case = ConflationUseCase::new(mock_output.clone());
        
        let enriched_record = create_test_enriched_record();
        let result = use_case.conflate_record(&enriched_record).await;

        assert!(result.is_ok());
        let conflated_record = result.unwrap();
        
        // Check that it was written to output
        let written_records = mock_output.get_written_records();
        assert_eq!(written_records.len(), 1);
        assert_eq!(written_records[0].canonical_entity_id, conflated_record.canonical_entity_id);
        
        // Check basic conflation properties
        assert_eq!(conflated_record.canonical_entity_id.entity_type, EntityType::Venue);
        assert_eq!(conflated_record.conflation.resolution_decision, ResolutionDecision::NewEntity);
        assert!(conflated_record.conflation.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_conflate_batch() {
        let mock_output = Arc::new(MockConflationOutputPort::new());
        let use_case = ConflationUseCase::new(mock_output.clone());
        
        let records = vec![
            create_test_enriched_record(),
            create_test_enriched_record(),
        ];
        
        let result = use_case.conflate_batch(&records).await;

        assert!(result.is_ok());
        let conflated_records = result.unwrap();
        assert_eq!(conflated_records.len(), 2);
        
        // Check that all were written to output
        let written_records = mock_output.get_written_records();
        assert_eq!(written_records.len(), 2);
    }

    #[tokio::test]
    async fn test_entity_type_determination() {
        let mock_output = Arc::new(MockConflationOutputPort::new());
        let use_case = ConflationUseCase::new(mock_output);
        
        let enriched_record = create_test_enriched_record();
        let entity_type = use_case.determine_entity_type_string(&enriched_record);
        
        assert_eq!(entity_type, "venue");
    }
}
