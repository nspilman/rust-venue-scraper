use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::pipeline::processing::enrich::EnrichedRecord;

/// A conflated record that represents a stable canonical entity after entity resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflatedRecord {
    /// The canonical entity ID that this record resolves to
    pub canonical_entity_id: EntityId,
    /// The enriched record that was processed
    pub enriched_record: EnrichedRecord,
    /// Conflation metadata about the resolution process
    pub conflation: ConflationMetadata,
    /// When this conflation was performed
    pub conflated_at: DateTime<Utc>,
}

/// A stable, durable entity identifier that persists across data updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId {
    /// The unique identifier for this canonical entity
    pub id: Uuid,
    /// The type of entity (venue, event, artist)
    pub entity_type: EntityType,
    /// Version of this entity (increments with updates)
    pub version: u64,
}

/// Types of entities that can be conflated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Venue,
    Event,
    Artist,
}

/// Metadata about the conflation process and entity resolution decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflationMetadata {
    /// The resolution decision made during conflation
    pub resolution_decision: ResolutionDecision,
    /// Confidence in the entity resolution (0.0 to 1.0)
    pub confidence: f64,
    /// The strategy/algorithm used for conflation
    pub strategy: String,
    /// Alternative entities that were considered but not matched
    pub alternatives: Vec<AlternativeMatch>,
    /// Previous entity ID if this is an update to existing entity
    pub previous_entity_id: Option<EntityId>,
    /// Source identifiers that contributed to this canonical entity
    pub contributing_sources: Vec<String>,
    /// Similarity scores with matched entities
    pub similarity_scores: HashMap<String, f64>,
    /// Warnings or notes from the conflation process
    pub warnings: Vec<String>,
    /// Deduplication metadata
    pub deduplication: DeduplicationMetadata,
}

/// The decision made during entity resolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolutionDecision {
    /// This is a new, previously unseen entity
    NewEntity,
    /// This matches an existing canonical entity (ID provided)
    MatchedExisting(EntityId),
    /// This updates an existing entity with new information
    UpdatedExisting(EntityId),
    /// This is a duplicate of an existing entity (no new information)
    Duplicate(EntityId),
    /// Conflation was uncertain - manual review may be needed
    Uncertain,
}

/// Information about alternative matches that were considered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeMatch {
    /// The entity ID that was considered as a match
    pub entity_id: EntityId,
    /// Similarity score with this alternative (0.0 to 1.0)
    pub similarity_score: f64,
    /// Reason this alternative was not selected
    pub rejection_reason: String,
}

/// Metadata about deduplication analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationMetadata {
    /// Whether this record was identified as a potential duplicate
    pub is_potential_duplicate: bool,
    /// Entity IDs of potential duplicates
    pub potential_duplicates: Vec<EntityId>,
    /// The deduplication strategy used
    pub deduplication_strategy: String,
    /// Key attributes used for deduplication matching
    pub key_attributes: Vec<String>,
    /// Hash or signature used for duplicate detection
    pub deduplication_signature: Option<String>,
}

/// Conflation configuration and matching rules
#[derive(Debug, Clone)]
pub struct ConflationConfig {
    /// Minimum confidence threshold for automatic matching
    pub min_confidence_threshold: f64,
    /// Matching strategies enabled (currently unused)
    #[allow(dead_code)]
    pub enabled_strategies: Vec<MatchingStrategy>,
    /// Maximum distance in km for venue location matching
    pub max_venue_distance_km: f64,
    /// Maximum time difference for event matching (in hours)
    pub max_event_time_diff_hours: i64,
/// Similarity threshold for text matching (names, descriptions) (currently unused)
    #[allow(dead_code)]
    pub text_similarity_threshold: f64,
}

/// Available matching strategies for entity resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchingStrategy {
    /// Exact name and location matching
    ExactMatch,
    /// Fuzzy text similarity matching
    FuzzyTextMatch,
    /// Geographic proximity matching (for venues)
    LocationProximity,
    /// Time-based matching (for events)
    TemporalProximity,
    /// Composite scoring across multiple attributes
    CompositeScore,
}

/// Trait for performing entity conflation and resolution
pub trait Conflator {
    /// Conflate an enriched record, resolving it to a canonical entity
    fn conflate(&self, record: &EnrichedRecord) -> anyhow::Result<ConflatedRecord>;
    
    /// Find potential matches for an enriched record
    fn find_potential_matches(&self, record: &EnrichedRecord) -> anyhow::Result<Vec<PotentialMatch>>;
    
    /// Calculate similarity between two records
    fn calculate_similarity(&self, record1: &EnrichedRecord, record2: &EnrichedRecord) -> f64;
}

/// A potential match found during conflation
#[derive(Debug, Clone)]
pub struct PotentialMatch {
    /// The entity ID of the potential match
    pub entity_id: EntityId,
    /// Similarity score (0.0 to 1.0)
    pub similarity_score: f64,
    /// Breakdown of similarity by attribute (currently unused)
    #[allow(dead_code)]
    pub similarity_breakdown: HashMap<String, f64>,
    /// The enriched record of the potential match (currently unused)
    #[allow(dead_code)]
    pub matched_record: EnrichedRecord,
}

/// Default conflator implementation for Seattle music venues
pub struct DefaultConflator {
    /// Configuration for conflation behavior
    pub config: ConflationConfig,
    /// In-memory store of known canonical entities for matching
    /// In production, this would be backed by a database
    pub entity_store: HashMap<EntityId, ConflatedRecord>,
    /// Lookup index for fast matching by key attributes
    pub name_index: HashMap<String, Vec<EntityId>>,
    pub location_index: HashMap<String, Vec<EntityId>>,
}

impl Default for DefaultConflator {
    fn default() -> Self {
        Self {
            config: ConflationConfig {
                min_confidence_threshold: 0.8,
                enabled_strategies: vec![
                    MatchingStrategy::ExactMatch,
                    MatchingStrategy::FuzzyTextMatch,
                    MatchingStrategy::LocationProximity,
                    MatchingStrategy::CompositeScore,
                ],
                max_venue_distance_km: 0.1, // 100 meters
                max_event_time_diff_hours: 2,
                text_similarity_threshold: 0.85,
            },
            entity_store: HashMap::new(),
            name_index: HashMap::new(),
            location_index: HashMap::new(),
        }
    }
}

impl DefaultConflator {
    /// Create a new conflator with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract entity name from enriched record
    fn extract_entity_name(&self, record: &EnrichedRecord) -> Option<String> {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        match &record.quality_assessed_record.normalized_record.entity {
            NormalizedEntity::Venue(venue) => Some(venue.name.clone()),
            NormalizedEntity::Event(event) => Some(event.title.clone()),
            NormalizedEntity::Artist(artist) => Some(artist.name.clone()),
        }
    }
    
    /// Extract location key for indexing venues
    fn extract_location_key(&self, record: &EnrichedRecord) -> Option<String> {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        if let NormalizedEntity::Venue(venue) = &record.quality_assessed_record.normalized_record.entity {
            // Create a location key based on coordinates rounded to ~100m precision
            let lat_key = (venue.latitude * 1000.0).round() as i32;
            let lng_key = (venue.longitude * 1000.0).round() as i32;
            Some(format!("{}_{}", lat_key, lng_key))
        } else {
            None
        }
    }
    
    /// Normalize entity name for consistent matching
    fn normalize_name(&self, name: &str) -> String {
        name.to_lowercase()
            .trim()
            .replace("&", "and")
            .replace("-", " ")
            .replace("_", " ")
            .replace("  ", " ")
            .to_string()
    }
    
    /// Calculate text similarity using simple token-based approach
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f64 {
        let normalized1 = self.normalize_name(text1);
        let normalized2 = self.normalize_name(text2);
        
        if normalized1 == normalized2 {
            return 1.0;
        }
        
        let tokens1: std::collections::HashSet<&str> = normalized1.split_whitespace().collect();
        let tokens2: std::collections::HashSet<&str> = normalized2.split_whitespace().collect();
        
        if tokens1.is_empty() && tokens2.is_empty() {
            return 1.0;
        }
        
        let intersection = tokens1.intersection(&tokens2).count();
        let union = tokens1.union(&tokens2).count();
        
        intersection as f64 / union as f64
    }
    
    /// Calculate distance between two coordinates in kilometers
    fn calculate_distance(&self, lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
        // Simple Euclidean distance for small distances (good enough for city-scale)
        let lat_diff = (lat1 - lat2) * 111.0; // ~111km per degree latitude
        let lng_diff = (lng1 - lng2) * 85.0;  // ~85km per degree longitude at Seattle's latitude
        (lat_diff * lat_diff + lng_diff * lng_diff).sqrt()
    }
    
    /// Generate a new canonical entity ID
    fn generate_entity_id(&self, entity_type: EntityType) -> EntityId {
        EntityId {
            id: Uuid::new_v4(),
            entity_type,
            version: 1,
        }
    }
    
    /// Generate entity ID, preserving existing ID if present in normalized entity
    fn generate_entity_id_from_record(&self, record: &EnrichedRecord) -> EntityId {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        let entity_type = self.determine_entity_type(record);
        
        // Check if the normalized entity already has an ID assigned
        let existing_id = match &record.quality_assessed_record.normalized_record.entity {
            NormalizedEntity::Artist(artist) => artist.id,
            NormalizedEntity::Event(event) => event.id,
            NormalizedEntity::Venue(venue) => venue.id,
        };
        
        EntityId {
            id: existing_id.unwrap_or_else(Uuid::new_v4),
            entity_type,
            version: 1,
        }
    }
    
    /// Determine entity type from enriched record
    fn determine_entity_type(&self, record: &EnrichedRecord) -> EntityType {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        match &record.quality_assessed_record.normalized_record.entity {
            NormalizedEntity::Venue(_) => EntityType::Venue,
            NormalizedEntity::Event(_) => EntityType::Event,
            NormalizedEntity::Artist(_) => EntityType::Artist,
        }
    }
    
    /// Create deduplication signature for the record
    fn create_deduplication_signature(&self, record: &EnrichedRecord) -> String {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        match &record.quality_assessed_record.normalized_record.entity {
            NormalizedEntity::Venue(venue) => {
                venue.name_lower.hash(&mut hasher);
                venue.city.hash(&mut hasher);
                venue.postal_code.hash(&mut hasher);
                // Round coordinates to ~10m precision for signature
                ((venue.latitude * 10000.0).round() as i64).hash(&mut hasher);
                ((venue.longitude * 10000.0).round() as i64).hash(&mut hasher);
            }
            NormalizedEntity::Event(event) => {
                event.title.to_lowercase().hash(&mut hasher);
                event.event_day.hash(&mut hasher);
                // Events don't have venue_name field directly - they have venue_id
                event.venue_id.hash(&mut hasher);
            }
            NormalizedEntity::Artist(artist) => {
                artist.name.to_lowercase().hash(&mut hasher);
                // Artists are harder to deduplicate - might need manual review
            }
        }
        
        format!("{:x}", hasher.finish())
    }
}

impl Conflator for DefaultConflator {
    fn conflate(&self, record: &EnrichedRecord) -> anyhow::Result<ConflatedRecord> {
        // Per-record metric: start processing
        crate::observability::metrics::conflation::records_processed();

        let conflated_at = Utc::now();
        let entity_type = self.determine_entity_type(record);
        let mut warnings = Vec::new();
        
        // Find potential matches
        let potential_matches = match self.find_potential_matches(record) {
            Ok(m) => m,
            Err(e) => {
                // Per-record metric: failure during matching
                crate::observability::metrics::conflation::records_failed();
                return Err(e);
            }
        };
        
        // Determine resolution decision based on potential matches
        let (resolution_decision, canonical_entity_id, confidence) = if potential_matches.is_empty() {
            // No matches found - create new entity, preserving ID if already set
            let new_id = self.generate_entity_id_from_record(record);
            (ResolutionDecision::NewEntity, new_id, 1.0)
        } else {
            // Find best match
            let best_match = potential_matches
                .iter()
                .max_by(|a, b| a.similarity_score.partial_cmp(&b.similarity_score).unwrap())
                .unwrap();
                
            if best_match.similarity_score >= self.config.min_confidence_threshold {
                // High confidence match
                (
                    ResolutionDecision::MatchedExisting(best_match.entity_id.clone()),
                    best_match.entity_id.clone(),
                    best_match.similarity_score,
                )
            } else {
                // Low confidence - treat as new entity but add warning, preserving ID if already set
                let new_id = self.generate_entity_id_from_record(record);
                warnings.push(format!(
                    "Potential matches found but confidence too low (best: {:.2})",
                    best_match.similarity_score
                ));
                (ResolutionDecision::NewEntity, new_id, 0.6)
            }
        };
        
        // Create alternatives list
        let alternatives: Vec<AlternativeMatch> = potential_matches
            .iter()
            .filter(|m| m.entity_id != canonical_entity_id)
            .map(|m| AlternativeMatch {
                entity_id: m.entity_id.clone(),
                similarity_score: m.similarity_score,
                rejection_reason: if m.similarity_score < self.config.min_confidence_threshold {
                    "Similarity score below threshold".to_string()
                } else {
                    "Lower similarity than selected match".to_string()
                },
            })
            .collect();
        
        // Create similarity scores map
        let similarity_scores: HashMap<String, f64> = potential_matches
            .iter()
            .map(|m| (m.entity_id.id.to_string(), m.similarity_score))
            .collect();
        
        // Create deduplication metadata
        let deduplication_signature = self.create_deduplication_signature(record);
        let is_potential_duplicate = potential_matches.len() > 1;
        let potential_duplicates: Vec<EntityId> = potential_matches
            .iter()
            .filter(|m| m.similarity_score > 0.9) // High similarity suggests duplication
            .map(|m| m.entity_id.clone())
            .collect();
        
        let deduplication = DeduplicationMetadata {
            is_potential_duplicate,
            potential_duplicates,
            deduplication_strategy: "signature_based".to_string(),
            key_attributes: vec!["name".to_string(), "location".to_string(), "date".to_string()],
            deduplication_signature: Some(deduplication_signature),
        };
        
        // Extract contributing sources
        let contributing_sources = vec![
            record.quality_assessed_record.normalized_record.provenance.source_id.clone()
        ];
        
        let conflation = ConflationMetadata {
            resolution_decision: resolution_decision.clone(),
            confidence,
            strategy: "default_conflator_v1".to_string(),
            alternatives: alternatives.clone(),
            previous_entity_id: None, // Would be set for updates
            contributing_sources,
            similarity_scores,
            warnings: warnings.clone(),
            deduplication: deduplication.clone(),
        };

        // Per-record metrics after decision
        crate::observability::metrics::conflation::confidence_score_recorded(confidence);
        match &resolution_decision {
            ResolutionDecision::NewEntity => crate::observability::metrics::conflation::new_entity_created(),
            ResolutionDecision::MatchedExisting(_) => crate::observability::metrics::conflation::matched_existing(),
            ResolutionDecision::UpdatedExisting(_) => crate::observability::metrics::conflation::updated_existing(),
            ResolutionDecision::Duplicate(_) => crate::observability::metrics::conflation::duplicate_detected(),
            ResolutionDecision::Uncertain => crate::observability::metrics::conflation::uncertain_resolution(),
        }
        if !warnings.is_empty() {
            for w in &warnings {
                crate::observability::metrics::conflation::warning_logged(w);
            }
        }
        crate::observability::metrics::conflation::alternative_matches(alternatives.len());
        crate::observability::metrics::conflation::potential_duplicates(deduplication.potential_duplicates.len());
        crate::observability::metrics::conflation::records_successful();
        
        Ok(ConflatedRecord {
            canonical_entity_id,
            enriched_record: record.clone(),
            conflation,
            conflated_at,
        })
    }
    
    fn find_potential_matches(&self, record: &EnrichedRecord) -> anyhow::Result<Vec<PotentialMatch>> {
        let mut potential_matches = Vec::new();
        
        // Get entity name for matching
        let entity_name = self.extract_entity_name(record);
        
        // Find matches by name
        if let Some(name) = entity_name {
            let normalized_name = self.normalize_name(&name);
            
            if let Some(entity_ids) = self.name_index.get(&normalized_name) {
                for entity_id in entity_ids {
                    if let Some(canonical_record) = self.entity_store.get(entity_id) {
                        let similarity_score = self.calculate_similarity(record, &canonical_record.enriched_record);
                        
                        if similarity_score > 0.3 { // Minimum threshold for consideration
                            let mut similarity_breakdown = HashMap::new();
                            similarity_breakdown.insert("name".to_string(), 
                                self.calculate_text_similarity(&name, 
                                    &self.extract_entity_name(&canonical_record.enriched_record).unwrap_or_default()));
                            
                            potential_matches.push(PotentialMatch {
                                entity_id: entity_id.clone(),
                                similarity_score,
                                similarity_breakdown,
                                matched_record: canonical_record.enriched_record.clone(),
                            });
                        }
                    }
                }
            }
        }
        
        // Find matches by location (for venues)
        if self.determine_entity_type(record) == EntityType::Venue {
            if let Some(location_key) = self.extract_location_key(record) {
                if let Some(entity_ids) = self.location_index.get(&location_key) {
                    for entity_id in entity_ids {
                        // Skip if already found by name
                        if potential_matches.iter().any(|m| m.entity_id == *entity_id) {
                            continue;
                        }
                        
                        if let Some(canonical_record) = self.entity_store.get(entity_id) {
                            let similarity_score = self.calculate_similarity(record, &canonical_record.enriched_record);
                            
                            if similarity_score > 0.5 { // Higher threshold for location-only matches
                                let mut similarity_breakdown = HashMap::new();
                                similarity_breakdown.insert("location".to_string(), similarity_score);
                                
                                potential_matches.push(PotentialMatch {
                                    entity_id: entity_id.clone(),
                                    similarity_score,
                                    similarity_breakdown,
                                    matched_record: canonical_record.enriched_record.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by similarity score (highest first)
        potential_matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        
        Ok(potential_matches)
    }
    
    fn calculate_similarity(&self, record1: &EnrichedRecord, record2: &EnrichedRecord) -> f64 {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        let entity1 = &record1.quality_assessed_record.normalized_record.entity;
        let entity2 = &record2.quality_assessed_record.normalized_record.entity;
        
        // Entity types must match
        if std::mem::discriminant(entity1) != std::mem::discriminant(entity2) {
            return 0.0;
        }
        
        match (entity1, entity2) {
            (NormalizedEntity::Venue(v1), NormalizedEntity::Venue(v2)) => {
                let name_similarity = self.calculate_text_similarity(&v1.name, &v2.name);
                let location_distance = self.calculate_distance(v1.latitude, v1.longitude, v2.latitude, v2.longitude);
                let location_similarity = if location_distance <= self.config.max_venue_distance_km {
                    1.0 - (location_distance / self.config.max_venue_distance_km).min(1.0)
                } else {
                    0.0
                };
                
                let address_similarity = self.calculate_text_similarity(&v1.address, &v2.address);
                
                // Weighted average: name (40%), location (40%), address (20%)
                (name_similarity * 0.4) + (location_similarity * 0.4) + (address_similarity * 0.2)
            }
            
            (NormalizedEntity::Event(e1), NormalizedEntity::Event(e2)) => {
                let name_similarity = self.calculate_text_similarity(&e1.title, &e2.title);
                
                // Date similarity - convert NaiveDate to datetime for comparison
                let date_diff = (e1.event_day.and_hms_opt(0, 0, 0).unwrap() - e2.event_day.and_hms_opt(0, 0, 0).unwrap()).num_hours().abs();
                let date_similarity = if date_diff <= self.config.max_event_time_diff_hours {
                    1.0 - (date_diff as f64 / self.config.max_event_time_diff_hours as f64)
                } else {
                    0.0
                };
                
                // Venue similarity based on venue_id match
                let venue_similarity = if e1.venue_id == e2.venue_id {
                    1.0 // Same venue
                } else {
                    0.0 // Different venues
                };
                
                // Weighted average: name (50%), date (30%), venue (20%)
                (name_similarity * 0.5) + (date_similarity * 0.3) + (venue_similarity * 0.2)
            }
            
            (NormalizedEntity::Artist(a1), NormalizedEntity::Artist(a2)) => {
                // For artists, primarily rely on name similarity
                // Could be enhanced with genre, biography text, etc.
                self.calculate_text_similarity(&a1.name, &a2.name)
            }
            
            _ => 0.0, // Different entity types
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sms_core::domain::{Event, Venue, Artist};
    use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
    use crate::pipeline::processing::quality_gate::{QualityAssessment, QualityDecision};
    use crate::pipeline::processing::enrich::{EnrichmentMetadata, GeoProperties, PopulationDensity, ReferenceVersions};
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    fn create_test_venue_record(name: &str, lat: f64, lng: f64) -> EnrichedRecord {
        let venue = Venue {
            id: None,
            name: name.to_string(),
            name_lower: name.to_lowercase(),
            slug: name.to_lowercase().replace(" ", "-"),
            latitude: lat,
            longitude: lng,
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

        let normalized_record = NormalizedRecord {
            entity: NormalizedEntity::Venue(venue),
            provenance: RecordProvenance {
                envelope_id: "test_envelope".to_string(),
                source_id: "test_source".to_string(),
                payload_ref: "test_payload".to_string(),
                record_path: "$.venues[0]".to_string(),
                normalized_at: Utc::now(),
            },
            normalization: NormalizationMetadata {
                confidence: 1.0,
                warnings: Vec::new(),
                geocoded: false,
                strategy: "test".to_string(),
            },
        };

        let quality_assessment = QualityAssessment {
            decision: QualityDecision::Accept,
            quality_score: 0.95,
            issues: Vec::new(),
            rule_version: "v1.0.0".to_string(),
        };

        let quality_assessed_record = crate::pipeline::processing::quality_gate::QualityAssessedRecord {
            normalized_record,
            quality_assessment,
            assessed_at: Utc::now(),
        };

        let enrichment = EnrichmentMetadata {
            city: Some("Seattle".to_string()),
            district: Some("Test District".to_string()),
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

    #[test]
    fn test_conflation_new_entity() {
        let conflator = DefaultConflator::new();
        let record = create_test_venue_record("Test Venue", 47.6131, -122.3424);
        
        let result = conflator.conflate(&record).unwrap();
        
        assert_eq!(result.conflation.resolution_decision, ResolutionDecision::NewEntity);
        assert_eq!(result.canonical_entity_id.entity_type, EntityType::Venue);
        assert_eq!(result.canonical_entity_id.version, 1);
    }

    #[test]
    fn test_text_similarity_calculation() {
        let conflator = DefaultConflator::new();
        
        // Exact match
        assert_eq!(conflator.calculate_text_similarity("Test Venue", "Test Venue"), 1.0);
        
        // Case insensitive
        assert_eq!(conflator.calculate_text_similarity("Test Venue", "test venue"), 1.0);
        
        // Partial match
        let similarity = conflator.calculate_text_similarity("Blue Moon Tavern", "Blue Moon");
        assert!(similarity > 0.0 && similarity < 1.0);
        
        // No match
        assert_eq!(conflator.calculate_text_similarity("Blue Moon", "Red Sun"), 0.0);
    }

    #[test]
    fn test_venue_distance_calculation() {
        let conflator = DefaultConflator::new();
        
        // Same location
        let distance = conflator.calculate_distance(47.6131, -122.3424, 47.6131, -122.3424);
        assert_eq!(distance, 0.0);
        
        // Different locations (should be > 0)
        let distance = conflator.calculate_distance(47.6131, -122.3424, 47.6200, -122.3500);
        assert!(distance > 0.0);
    }

    #[test]
    fn test_entity_type_determination() {
        let conflator = DefaultConflator::new();
        let venue_record = create_test_venue_record("Test Venue", 47.6131, -122.3424);
        
        let entity_type = conflator.determine_entity_type(&venue_record);
        assert_eq!(entity_type, EntityType::Venue);
    }
}
