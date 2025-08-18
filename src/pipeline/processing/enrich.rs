use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::pipeline::processing::quality_gate::QualityAssessedRecord;

/// An enriched record that has passed through quality gate and been enhanced
/// with contextual information like spatial bins, city tags, and routing labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedRecord {
    /// The original quality-assessed record
    pub quality_assessed_record: QualityAssessedRecord,
    /// Enrichment metadata and contextual additions
    pub enrichment: EnrichmentMetadata,
    /// When this enrichment was performed
    pub enriched_at: DateTime<Utc>,
}

/// Enrichment metadata containing contextual additions to the record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentMetadata {
    /// The city or municipal area this record belongs to
    pub city: Option<String>,
    /// District or neighborhood within the city
    pub district: Option<String>,
    /// Administrative region (county, state, etc.)
    pub region: Option<String>,
    /// Spatial bin for quick geographic lookups (e.g., "seattle_grid_42_13")
    pub spatial_bin: Option<String>,
    /// Tags for partitioning and routing (e.g., ["music", "nightlife", "downtown"])
    pub tags: Vec<String>,
    /// Computed geographical properties
    pub geo_properties: GeoProperties,
    /// Reference data versions used for enrichment
    pub reference_versions: ReferenceVersions,
    /// The enrichment strategy used
    pub strategy: String,
    /// Confidence in the enrichment (0.0 to 1.0)
    pub confidence: f64,
    /// Any warnings from the enrichment process
    pub warnings: Vec<String>,
}

/// Geographical properties computed during enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoProperties {
    /// Whether coordinates are within expected city bounds
    pub within_city_bounds: bool,
    /// Distance from city center in kilometers
    pub distance_from_center_km: Option<f64>,
    /// Population density category for the area
    pub population_density: PopulationDensity,
    /// Transit accessibility score (0.0 to 1.0)
    pub transit_accessibility: Option<f64>,
    /// Nearby landmarks or points of interest
    pub nearby_landmarks: Vec<String>,
}

/// Population density categories for spatial enrichment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PopulationDensity {
    /// Rural or very low density
    Rural,
    /// Suburban or low density
    Suburban,
    /// Urban or medium density
    Urban,
    /// Dense urban or high density
    Dense,
    /// Unknown density
    Unknown,
}

/// Reference data versions used during enrichment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceVersions {
    /// City boundaries data version
    pub city_boundaries: Option<String>,
    /// Administrative boundaries version
    pub admin_boundaries: Option<String>,
    /// Spatial grid version
    pub spatial_grid: Option<String>,
    /// Points of interest data version
    pub poi_data: Option<String>,
}

/// Trait for enriching quality-assessed records with contextual information
pub trait Enricher {
    /// Enrich a quality-assessed record with contextual metadata
    fn enrich(&self, record: &QualityAssessedRecord) -> anyhow::Result<EnrichedRecord>;
}

/// Default enricher that adds Seattle-area specific contextual information
pub struct DefaultEnricher {
    /// Seattle city center coordinates for distance calculations
    pub city_center: (f64, f64), // lat, lng
    /// Spatial grid size in degrees for binning
    pub spatial_grid_size: f64,
    /// Reference data versions
    pub reference_versions: ReferenceVersions,
}

impl Default for DefaultEnricher {
    fn default() -> Self {
        Self {
            city_center: (47.6062, -122.3321), // Seattle center
            spatial_grid_size: 0.01, // ~1km grid
            reference_versions: ReferenceVersions {
                city_boundaries: Some("seattle_v1.0".to_string()),
                admin_boundaries: Some("wa_king_county_v1.0".to_string()),
                spatial_grid: Some("grid_1km_v1.0".to_string()),
                poi_data: Some("seattle_poi_v1.0".to_string()),
            },
        }
    }
}

impl DefaultEnricher {
    /// Create a new enricher with default Seattle configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an enricher with custom city center
    pub fn with_city_center(lat: f64, lng: f64) -> Self {
        Self {
            city_center: (lat, lng),
            ..Default::default()
        }
    }

    /// Calculate distance between two coordinates in kilometers
    fn calculate_distance(&self, lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
        // Simple Euclidean distance for small distances (good enough for city-scale)
        // For production, consider using Haversine formula
        let lat_diff = (lat1 - lat2) * 111.0; // ~111km per degree latitude
        let lng_diff = (lng1 - lng2) * 85.0;  // ~85km per degree longitude at Seattle's latitude
        (lat_diff * lat_diff + lng_diff * lng_diff).sqrt()
    }

    /// Generate spatial bin identifier for coordinates
    fn generate_spatial_bin(&self, lat: f64, lng: f64) -> String {
        let grid_lat = (lat / self.spatial_grid_size).floor() as i32;
        let grid_lng = (lng / self.spatial_grid_size).floor() as i32;
        format!("seattle_grid_{}_{}", grid_lat, grid_lng)
    }

    /// Determine if coordinates are within Seattle city bounds (simplified)
    fn is_within_city_bounds(&self, lat: f64, lng: f64) -> bool {
        // Simple bounding box for Seattle (production would use proper polygons)
        lat >= 47.48 && lat <= 47.74 && lng >= -122.46 && lng <= -122.22
    }

    /// Classify population density based on distance from city center
    fn classify_population_density(&self, distance_km: f64) -> PopulationDensity {
        match distance_km {
            d if d < 5.0 => PopulationDensity::Dense,
            d if d < 15.0 => PopulationDensity::Urban,
            d if d < 30.0 => PopulationDensity::Suburban,
            _ => PopulationDensity::Rural,
        }
    }

    /// Estimate transit accessibility (simplified scoring)
    fn estimate_transit_accessibility(&self, distance_km: f64) -> f64 {
        // Simple model: closer to city center = better transit
        let max_distance = 50.0;
        let normalized_distance = (distance_km / max_distance).min(1.0);
        (1.0 - normalized_distance).max(0.0)
    }

    /// Find nearby landmarks (simplified implementation)
    fn find_nearby_landmarks(&self, lat: f64, lng: f64) -> Vec<String> {
        let mut landmarks = Vec::new();
        
        // Hardcoded Seattle landmarks for demo
        let seattle_landmarks = vec![
            (47.6205, -122.3493, "Space Needle"),
            (47.6089, -122.3356, "Pike Place Market"), 
            (47.5952, -122.3316, "Pioneer Square"),
            (47.6040, -122.3349, "Waterfront"),
            (47.6131, -122.3424, "Belltown"),
            (47.6249, -122.3364, "Capitol Hill"),
            (47.6690, -122.3847, "Ballard"),
            (47.6815, -122.3534, "Fremont"),
        ];

        for (landmark_lat, landmark_lng, name) in seattle_landmarks {
            let distance = self.calculate_distance(lat, lng, landmark_lat, landmark_lng);
            if distance < 2.0 { // Within 2km
                landmarks.push(name.to_string());
            }
        }

        landmarks
    }

    /// Determine city from coordinates (simplified)
    fn determine_city(&self, lat: f64, lng: f64) -> Option<String> {
        if self.is_within_city_bounds(lat, lng) {
            Some("Seattle".to_string())
        } else {
            None
        }
    }

    /// Determine district/neighborhood (simplified)
    fn determine_district(&self, lat: f64, lng: f64) -> Option<String> {
        // Simplified neighborhood detection based on coordinates
        match (lat, lng) {
            (lat, lng) if lat > 47.62 && lng > -122.34 => Some("Capitol Hill".to_string()),
            (lat, lng) if lat > 47.66 && lng < -122.37 => Some("Ballard".to_string()),
            (lat, lng) if lat > 47.61 && lng > -122.34 => Some("Belltown".to_string()),
            (lat, lng) if lat < 47.60 && lng > -122.34 => Some("Pioneer Square".to_string()),
            (lat, lng) if lat > 47.67 && lng > -122.36 => Some("Fremont".to_string()),
            _ if self.is_within_city_bounds(lat, lng) => Some("Seattle".to_string()),
            _ => None,
        }
    }

    /// Generate contextual tags based on entity type and location
    fn generate_tags(&self, record: &QualityAssessedRecord, district: Option<&str>) -> Vec<String> {
        let mut tags = Vec::new();

        // Add entity type tags
        use crate::pipeline::processing::normalize::NormalizedEntity;
        match &record.normalized_record.entity {
            NormalizedEntity::Event(_) => {
                tags.push("event".to_string());
                tags.push("music".to_string());
                tags.push("entertainment".to_string());
            }
            NormalizedEntity::Venue(_) => {
                tags.push("venue".to_string());
                tags.push("location".to_string());
                tags.push("entertainment".to_string());
            }
            NormalizedEntity::Artist(_) => {
                tags.push("artist".to_string());
                tags.push("performer".to_string());
                tags.push("music".to_string());
            }
        }

        // Add location-based tags
        if let Some(district) = district {
            tags.push(district.to_lowercase().replace(" ", "_"));
            
            // Add neighborhood-specific tags
            match district {
                "Capitol Hill" => {
                    tags.push("nightlife".to_string());
                    tags.push("arts".to_string());
                    tags.push("hipster".to_string());
                }
                "Ballard" => {
                    tags.push("craft_beer".to_string());
                    tags.push("maritime".to_string());
                }
                "Belltown" => {
                    tags.push("urban".to_string());
                    tags.push("upscale".to_string());
                }
                "Pioneer Square" => {
                    tags.push("historic".to_string());
                    tags.push("downtown".to_string());
                }
                _ => {}
            }
        }

        tags.push("seattle".to_string());
        tags.push("pacific_northwest".to_string());
        
        tags
    }

    /// Extract coordinates from a quality-assessed record
    fn extract_coordinates(&self, record: &QualityAssessedRecord) -> Option<(f64, f64)> {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        match &record.normalized_record.entity {
            NormalizedEntity::Venue(venue) => Some((venue.latitude, venue.longitude)),
            NormalizedEntity::Event(_) => {
                // Events don't have direct coordinates, but we could look up venue coordinates
                // For now, return None - this would be resolved during conflation
                None
            }
            NormalizedEntity::Artist(_) => None,
        }
    }
}

impl Enricher for DefaultEnricher {
    fn enrich(&self, record: &QualityAssessedRecord) -> anyhow::Result<EnrichedRecord> {
        let enriched_at = Utc::now();
        let mut warnings = Vec::new();
        
        // Extract coordinates if available
        let coordinates = self.extract_coordinates(record);
        
        let (city, district, geo_properties, spatial_bin) = if let Some((lat, lng)) = coordinates {
            let city = self.determine_city(lat, lng);
            let district = self.determine_district(lat, lng);
            let distance_from_center = self.calculate_distance(
                lat, lng,
                self.city_center.0, self.city_center.1
            );
            
            let within_bounds = self.is_within_city_bounds(lat, lng);
            if !within_bounds {
                warnings.push("Coordinates appear to be outside Seattle city bounds".to_string());
            }

            let geo_properties = GeoProperties {
                within_city_bounds: within_bounds,
                distance_from_center_km: Some(distance_from_center),
                population_density: self.classify_population_density(distance_from_center),
                transit_accessibility: Some(self.estimate_transit_accessibility(distance_from_center)),
                nearby_landmarks: self.find_nearby_landmarks(lat, lng),
            };

            let spatial_bin = Some(self.generate_spatial_bin(lat, lng));
            
            (city, district, geo_properties, spatial_bin)
        } else {
            // No coordinates available - limited enrichment
            warnings.push("No coordinates available for geographic enrichment".to_string());
            
            let geo_properties = GeoProperties {
                within_city_bounds: false,
                distance_from_center_km: None,
                population_density: PopulationDensity::Unknown,
                transit_accessibility: None,
                nearby_landmarks: Vec::new(),
            };
            
            (None, None, geo_properties, None)
        };

        // Generate contextual tags
        let tags = self.generate_tags(record, district.as_deref());

        // Calculate enrichment confidence
        let confidence = if coordinates.is_some() {
            let base_confidence = 0.9;
            let quality_penalty = (1.0 - record.quality_assessment.quality_score) * 0.2;
            let bounds_penalty = if geo_properties.within_city_bounds { 0.0 } else { 0.1 };
            (base_confidence - quality_penalty - bounds_penalty).max(0.3)
        } else {
            0.6 // Lower confidence without coordinates
        };

        let enrichment = EnrichmentMetadata {
            city,
            district,
            region: Some("King County, WA".to_string()),
            spatial_bin,
            tags,
            geo_properties,
            reference_versions: self.reference_versions.clone(),
            strategy: "default_seattle_enrichment".to_string(),
            confidence,
            warnings,
        };

        Ok(EnrichedRecord {
            quality_assessed_record: record.clone(),
            enrichment,
            enriched_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Event, Venue};
    use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
    use crate::pipeline::processing::quality_gate::{QualityAssessment, QualityDecision};
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    fn create_test_venue_record() -> QualityAssessedRecord {
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
                confidence: 0.9,
                warnings: Vec::new(),
                geocoded: false,
                strategy: "test".to_string(),
            },
        };

        QualityAssessedRecord {
            normalized_record,
            quality_assessment: QualityAssessment {
                decision: QualityDecision::Accept,
                quality_score: 0.85,
                issues: Vec::new(),
                rule_version: "v1.0.0".to_string(),
            },
            assessed_at: Utc::now(),
        }
    }

    #[test]
    fn test_enrich_venue_record() {
        let enricher = DefaultEnricher::new();
        let record = create_test_venue_record();

        let result = enricher.enrich(&record).unwrap();

        // Check basic enrichment
        assert_eq!(result.enrichment.city, Some("Seattle".to_string()));
        assert_eq!(result.enrichment.district, Some("Belltown".to_string()));
        assert!(result.enrichment.confidence > 0.7);
        
        // Check geo properties
        assert!(result.enrichment.geo_properties.within_city_bounds);
        assert!(result.enrichment.geo_properties.distance_from_center_km.is_some());
        assert_eq!(result.enrichment.geo_properties.population_density, PopulationDensity::Dense);

        // Check spatial binning
        assert!(result.enrichment.spatial_bin.is_some());
        assert!(result.enrichment.spatial_bin.unwrap().starts_with("seattle_grid_"));

        // Check tags
        let tags = &result.enrichment.tags;
        assert!(tags.contains(&"venue".to_string()));
        assert!(tags.contains(&"seattle".to_string()));
        assert!(tags.contains(&"belltown".to_string()));
        assert!(tags.contains(&"upscale".to_string()));
    }

    #[test]
    fn test_distance_calculation() {
        let enricher = DefaultEnricher::new();
        
        // Distance from Seattle center to roughly Capitol Hill
        let distance = enricher.calculate_distance(47.6062, -122.3321, 47.6249, -122.3120);
        assert!(distance > 1.0 && distance < 3.0); // Should be around 2km
    }

    #[test]
    fn test_spatial_binning() {
        let enricher = DefaultEnricher::new();
        let bin = enricher.generate_spatial_bin(47.6131, -122.3424);
        
        // Should generate a consistent grid identifier
        assert!(bin.starts_with("seattle_grid_"));
        assert!(bin.contains("4761")); // Grid cell for this latitude
    }
}
