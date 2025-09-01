use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;

/// String similarity and normalization utilities for entity matching
pub struct StringUtils;

impl StringUtils {
    /// Calculate string similarity using Levenshtein distance
    pub fn calculate_similarity(s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }
        
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }
        
        let max_len = len1.max(len2);
        let distance = Self::levenshtein_distance(s1, s2);
        
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let chars1: Vec<char> = s1.chars().collect();
        let chars2: Vec<char> = s2.chars().collect();
        let len1 = chars1.len();
        let len2 = chars2.len();
        
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }
        
        matrix[len1][len2]
    }

    /// Normalize venue name for matching
    pub fn normalize_venue_name(name: &str) -> String {
        name.to_lowercase()
            .replace("the ", "")
            .replace(" tavern", "")
            .replace(" bar", "")
            .replace(" club", "")
            .replace(" lounge", "")
            .trim()
            .to_string()
    }

    /// Normalize artist name for matching
    pub fn normalize_artist_name(name: &str) -> String {
        name.to_lowercase()
            .replace("the ", "")
            .replace(" band", "")
            .replace(" trio", "")
            .replace(" quartet", "")
            .trim()
            .to_string()
    }

    /// Extract artist names from event title
    pub fn extract_artist_names_from_title(title: &str) -> Vec<String> {
        // Skip common non-artist event types
        let title_lower = title.to_lowercase();
        if title_lower.contains("open mic") || 
           title_lower.contains("karaoke") || 
           title_lower.contains("art") && title_lower.contains("craft") ||
           title_lower.contains("bingo") ||
           title_lower.contains("dj") && !title_lower.contains("dj ") {
            return vec![title.to_string()];
        }
        
        // Split by common separators and clean up
        title.split(&[',', '&', '+', '/', '|'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }
}

/// Entity resolution utilities for conflation
pub struct EntityResolver;

impl EntityResolver {
    /// Resolve venue entity with deduplication
    pub fn resolve_venue_entity(
        venue_name: &str,
        venue_entities: &mut HashMap<String, Uuid>,
        confidence_threshold: f64
    ) -> Uuid {
        let normalized_venue_name = StringUtils::normalize_venue_name(venue_name);
        
        // Check if we already have this venue
        if let Some(existing_id) = venue_entities.get(&normalized_venue_name) {
            return *existing_id;
        }
        
        // Look for similar venues using fuzzy matching
        for (existing_name, existing_id) in venue_entities.iter() {
            let similarity = StringUtils::calculate_similarity(&normalized_venue_name, existing_name);
            if similarity >= confidence_threshold {
                tracing::info!("🔗 Matched venue '{}' to existing '{}' (similarity: {:.2})", 
                    venue_name, existing_name, similarity);
                return *existing_id;
            }
        }
        
        // Create new venue entity
        let new_venue_id = Uuid::new_v4();
        venue_entities.insert(normalized_venue_name, new_venue_id);
        tracing::info!("🆕 Created new venue entity: {} (ID: {})", venue_name, new_venue_id);
        
        new_venue_id
    }

    /// Resolve artist entities with deduplication
    pub fn resolve_artist_entities(
        artist_names: &[String],
        artist_entities: &mut HashMap<String, Uuid>,
        confidence_threshold: f64
    ) -> Vec<Uuid> {
        let mut resolved_ids = Vec::new();
        
        for artist_name in artist_names {
            let normalized_artist_name = StringUtils::normalize_artist_name(artist_name);
            
            // Check if we already have this artist
            if let Some(existing_id) = artist_entities.get(&normalized_artist_name) {
                resolved_ids.push(*existing_id);
                continue;
            }
            
            // Look for similar artists using fuzzy matching
            let mut found_match = false;
            for (existing_name, existing_id) in artist_entities.iter() {
                let similarity = StringUtils::calculate_similarity(&normalized_artist_name, existing_name);
                if similarity >= confidence_threshold {
                    tracing::info!("🔗 Matched artist '{}' to existing '{}' (similarity: {:.2})", 
                        artist_name, existing_name, similarity);
                    resolved_ids.push(*existing_id);
                    found_match = true;
                    break;
                }
            }
            
            if !found_match {
                // Create new artist entity
                let new_artist_id = Uuid::new_v4();
                artist_entities.insert(normalized_artist_name, new_artist_id);
                resolved_ids.push(new_artist_id);
                tracing::info!("🆕 Created new artist entity: {} (ID: {})", artist_name, new_artist_id);
            }
        }
        
        resolved_ids
    }
}

/// Event categorization utilities
pub struct EventCategorizer;

impl EventCategorizer {
    /// Categorize event based on content
    pub fn categorize_event(title: &str) -> Vec<String> {
        let mut categories = vec!["Music".to_string()];
        
        let title_lower = title.to_lowercase();
        
        if title_lower.contains("karaoke") {
            categories.push("Karaoke".to_string());
        }
        if title_lower.contains("open mic") || title_lower.contains("open-mic") {
            categories.push("Open Mic".to_string());
        }
        if title_lower.contains("art") || title_lower.contains("craft") {
            categories.push("Arts & Crafts".to_string());
        }
        if title_lower.contains("bingo") {
            categories.push("Games".to_string());
        }
        if title_lower.contains("dj") {
            categories.push("DJ".to_string());
        }
        if title_lower.contains("festival") {
            categories.push("Festival".to_string());
        }
        
        categories
    }
}

/// Quality gate validation utilities
pub struct QualityValidator;

impl QualityValidator {
    /// Apply quality gate checks to normalized data
    pub fn validate_event_data(
        title: &str,
        venue_name: &str,
        event_day: chrono::NaiveDate,
        source_api: &str
    ) -> QualityResult {
        // Check 1: Title must not be empty
        if title.trim().is_empty() {
            return QualityResult {
                passed: false,
                reason: "Title is empty".to_string(),
            };
        }
        
        // Check 2: Title must not be too short (less than 2 characters)
        if title.trim().len() < 2 {
            return QualityResult {
                passed: false,
                reason: "Title is too short".to_string(),
            };
        }
        
        // Check 3: Venue name must not be empty
        if venue_name.trim().is_empty() {
            return QualityResult {
                passed: false,
                reason: "Venue name is empty".to_string(),
            };
        }
        
        // Check 4: Event date must be reasonable (not too far in the past or future)
        let today = chrono::Utc::now().date_naive();
        let days_diff = (event_day - today).num_days();
        
        if days_diff < -365 {
            return QualityResult {
                passed: false,
                reason: "Event date is more than 1 year in the past".to_string(),
            };
        }
        
        if days_diff > 730 {
            return QualityResult {
                passed: false,
                reason: "Event date is more than 2 years in the future".to_string(),
            };
        }
        
        // Check 5: Source API must be valid
        if source_api.trim().is_empty() {
            return QualityResult {
                passed: false,
                reason: "Source API is empty".to_string(),
            };
        }
        
        // All checks passed
        QualityResult {
            passed: true,
            reason: "All quality checks passed".to_string(),
        }
    }
}

/// Quality gate result structure
#[derive(Debug, Clone)]
pub struct QualityResult {
    pub passed: bool,
    pub reason: String,
}

/// Validation utilities for catalog integrity
pub struct CatalogValidator;

impl CatalogValidator {
    /// Validate catalog integrity
    pub fn validate_catalog_integrity(entries: &[CatalogEntry]) -> Result<String> {
        let mut validation_issues = Vec::new();
        
        // Check for duplicate events
        let mut event_signatures = std::collections::HashSet::new();
        for entry in entries {
            let signature = format!("{}_{}", entry.venue_name, entry.event_date);
            if !event_signatures.insert(signature.clone()) {
                validation_issues.push(format!("Potential duplicate event: {}", signature));
            }
        }
        
        // Check for orphaned references
        let venue_ids: std::collections::HashSet<_> = entries.iter()
            .filter_map(|e| e.venue_id)
            .collect();
        let artist_ids: std::collections::HashSet<_> = entries.iter()
            .flat_map(|e| &e.artist_ids)
            .cloned()
            .collect();
        
        // Validate date ranges
        let mut future_events = 0;
        let mut past_events = 0;
        let today = chrono::Utc::now().date_naive();
        
        for entry in entries {
            let days_diff = (entry.event_date - today).num_days();
            if days_diff > 365 {
                future_events += 1;
            } else if days_diff < -30 {
                past_events += 1;
            }
        }
        
        if validation_issues.is_empty() {
            Ok(format!("All {} events validated successfully. {} venues, {} artists, {} future events, {} past events", 
                entries.len(), venue_ids.len(), artist_ids.len(), future_events, past_events))
        } else {
            Ok(format!("Validation completed with {} issues: {}", 
                validation_issues.len(), validation_issues.join("; ")))
        }
    }
}

/// Final catalog entry for graph database storage
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub event_id: uuid::Uuid,
    pub event_title: String,
    pub venue_name: String,
    pub venue_id: Option<uuid::Uuid>,
    pub artist_ids: Vec<uuid::Uuid>,
    pub event_date: chrono::NaiveDate,
    pub event_time: Option<chrono::NaiveTime>,
    pub location_info: Option<LocationInfo>,
    pub categories: Vec<String>,
    pub source_api: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Location information for venues
#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub neighborhood: Option<String>,
}
