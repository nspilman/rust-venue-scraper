use async_trait::async_trait;
use serde_json;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use crate::app::ports::ConflationOutputPort;
use crate::pipeline::processing::conflation::ConflatedRecord;

/// File-based adapter for writing conflated records to NDJSON files
/// Follows the same pattern as existing output adapters in the project
pub struct ConflationOutputAdapter {
    /// Base output directory for conflated records
    pub output_dir: PathBuf,
    /// Whether to create subdirectories by date
    pub use_date_partitioning: bool,
}

impl ConflationOutputAdapter {
    /// Create a new conflation output adapter
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            use_date_partitioning: true,
        }
    }

    /// Determine the output file path for a conflated record
    fn get_output_path(&self, record: &ConflatedRecord) -> PathBuf {
        let mut path = self.output_dir.clone();

        if self.use_date_partitioning {
            // Create date-based subdirectories: output_dir/year=YYYY/month=MM/day=DD/
            let _conflated_date = record.conflated_at.format("%Y-%m-%d");
            let year = record.conflated_at.format("%Y");
            let month = record.conflated_at.format("%m");
            let day = record.conflated_at.format("%d");
            
            path.push(format!("year={}", year));
            path.push(format!("month={}", month));
            path.push(format!("day={}", day));
        }

        // Create filename with timestamp and entity type for easier organization
        let filename = format!(
            "conflated-{}-{}.ndjson",
            record.canonical_entity_id.entity_type.to_string().to_lowercase(),
            record.conflated_at.format("%Y%m%d")
        );
        
        path.push(filename);
        path
    }

    /// Ensure the output directory exists
    async fn ensure_output_directory(&self, file_path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent_dir) = file_path.parent() {
            if !parent_dir.exists() {
                tokio::fs::create_dir_all(parent_dir).await
                    .map_err(|e| anyhow::anyhow!("Failed to create output directory {:?}: {}", parent_dir, e))?;
                
                debug!("Created output directory: {:?}", parent_dir);
            }
        }
        Ok(())
    }

    /// Convert conflated record to JSON line
    fn record_to_json_line(&self, record: &ConflatedRecord) -> anyhow::Result<String> {
        let json_record = serde_json::to_string(record)
            .map_err(|e| anyhow::anyhow!("Failed to serialize conflated record to JSON: {}", e))?;
        Ok(format!("{}\n", json_record))
    }
}

#[async_trait]
impl ConflationOutputPort for ConflationOutputAdapter {
    async fn write_conflated_record(&self, record: &ConflatedRecord) -> anyhow::Result<()> {
        let output_path = self.get_output_path(record);
        
        // Ensure output directory exists
        self.ensure_output_directory(&output_path).await?;
        
        // Convert record to JSON line
        let json_line = self.record_to_json_line(record)?;
        
        // Append to the file (create if doesn't exist)
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&output_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to open output file {:?}: {}", output_path, e))?;
        
        file.write_all(json_line.as_bytes()).await
            .map_err(|e| anyhow::anyhow!("Failed to write to output file {:?}: {}", output_path, e))?;
        
        file.flush().await
            .map_err(|e| anyhow::anyhow!("Failed to flush output file {:?}: {}", output_path, e))?;
        
        debug!(
            "Successfully wrote conflated record for entity {} to {:?}",
            record.canonical_entity_id.id,
            output_path
        );
        
        Ok(())
    }
}

// Helper trait to convert EntityType to string (this would normally be an impl block)
impl ToString for crate::pipeline::processing::conflation::EntityType {
    fn to_string(&self) -> String {
        match self {
            crate::pipeline::processing::conflation::EntityType::Venue => "venue".to_string(),
            crate::pipeline::processing::conflation::EntityType::Event => "event".to_string(),
            crate::pipeline::processing::conflation::EntityType::Artist => "artist".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Venue;
    use crate::pipeline::processing::conflation::{ConflatedRecord, ConflationMetadata, EntityId, EntityType, ResolutionDecision, DeduplicationMetadata};
    use crate::pipeline::processing::enrich::{EnrichedRecord, EnrichmentMetadata, GeoProperties, PopulationDensity, ReferenceVersions};
    use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
    use crate::pipeline::processing::quality_gate::{QualityAssessment, QualityDecision, QualityAssessedRecord};
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_conflated_record() -> ConflatedRecord {
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

        let enriched_record = EnrichedRecord {
            quality_assessed_record,
            enrichment,
            enriched_at: Utc::now(),
        };

        let canonical_entity_id = EntityId {
            id: Uuid::new_v4(),
            entity_type: EntityType::Venue,
            version: 1,
        };

        let conflation = ConflationMetadata {
            resolution_decision: ResolutionDecision::NewEntity,
            confidence: 0.95,
            strategy: "default_conflator_v1".to_string(),
            alternatives: Vec::new(),
            previous_entity_id: None,
            contributing_sources: vec!["test_source".to_string()],
            similarity_scores: HashMap::new(),
            warnings: Vec::new(),
            deduplication: DeduplicationMetadata {
                is_potential_duplicate: false,
                potential_duplicates: Vec::new(),
                deduplication_strategy: "signature_based".to_string(),
                key_attributes: vec!["name".to_string(), "location".to_string()],
                deduplication_signature: Some("test_signature".to_string()),
            },
        };

        ConflatedRecord {
            canonical_entity_id,
            enriched_record,
            conflation,
            conflated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_write_conflated_record() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = ConflationOutputAdapter::new(temp_dir.path().to_path_buf());
        
        let conflated_record = create_test_conflated_record();
        let result = adapter.write_conflated_record(&conflated_record).await;
        
        assert!(result.is_ok());
        
        // Verify file was created
        let output_path = adapter.get_output_path(&conflated_record);
        assert!(output_path.exists());
        
        // Verify file contents
        let contents = tokio::fs::read_to_string(&output_path).await.unwrap();
        assert!(!contents.is_empty());
        assert!(contents.contains("\"canonical_entity_id\""));
        assert!(contents.contains("\"resolution_decision\""));
    }

    #[tokio::test]
    async fn test_output_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = ConflationOutputAdapter::new(temp_dir.path().to_path_buf());
        
        let conflated_record = create_test_conflated_record();
        let output_path = adapter.get_output_path(&conflated_record);
        
        // Should include date partitioning
        let path_str = output_path.to_string_lossy();
        assert!(path_str.contains("year="));
        assert!(path_str.contains("month="));
        assert!(path_str.contains("day="));
        assert!(path_str.contains("conflated-venue-"));
        assert!(path_str.ends_with(".ndjson"));
    }

    #[tokio::test]
    async fn test_no_date_partitioning() {
        let temp_dir = TempDir::new().unwrap();
        let mut adapter = ConflationOutputAdapter::new(temp_dir.path().to_path_buf());
        adapter.use_date_partitioning = false;
        
        let conflated_record = create_test_conflated_record();
        let output_path = adapter.get_output_path(&conflated_record);
        
        // Should not include date partitioning
        let path_str = output_path.to_string_lossy();
        assert!(!path_str.contains("year="));
        assert!(!path_str.contains("month="));
        assert!(!path_str.contains("day="));
        assert!(path_str.ends_with(".ndjson"));
    }

    #[tokio::test]
    async fn test_json_serialization() {
        let temp_dir = TempDir::new().unwrap();
        let adapter = ConflationOutputAdapter::new(temp_dir.path().to_path_buf());
        
        let conflated_record = create_test_conflated_record();
        let json_line = adapter.record_to_json_line(&conflated_record).unwrap();
        
        // Should be valid JSON ending with newline
        assert!(json_line.ends_with('\n'));
        let json_part = json_line.trim_end();
        assert!(serde_json::from_str::<serde_json::Value>(json_part).is_ok());
    }
}
