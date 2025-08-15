use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::pipeline::processing::normalize::NormalizedRecord;

/// A quality-assessed record that has passed through the Quality Gate checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessedRecord {
    /// The original normalized record
    pub normalized_record: NormalizedRecord,
    /// The quality assessment result
    pub quality_assessment: QualityAssessment,
    /// When this quality assessment was performed
    pub assessed_at: DateTime<Utc>,
}

/// Quality assessment result from the Quality Gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// The quality gate decision
    pub decision: QualityDecision,
    /// Overall quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Specific quality issues found
    pub issues: Vec<QualityIssue>,
    /// The quality rule set version used
    pub rule_version: String,
}

/// Quality Gate decision for a record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QualityDecision {
    /// Record meets quality standards and proceeds to next stage
    Accept,
    /// Record has quality concerns but proceeds with warnings
    AcceptWithWarnings,
    /// Record fails quality checks and is quarantined for review
    Quarantine,
}

/// Individual quality issue found during assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    /// The type of quality issue
    pub issue_type: QualityIssueType,
    /// Severity level of the issue
    pub severity: QualitySeverity,
    /// Human-readable description of the issue
    pub description: String,
    /// Field or attribute that triggered this issue
    pub field: Option<String>,
    /// Expected or suggested value
    pub suggestion: Option<String>,
}

/// Types of quality issues that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityIssueType {
    /// Missing required data
    MissingData,
    /// Invalid format or structure
    InvalidFormat,
    /// Data outside expected ranges
    OutOfRange,
    /// Suspicious or anomalous values
    SuspiciousValue,
    /// Incomplete coordinate information
    IncompleteGeography,
    /// Date/time inconsistencies
    TemporalInconsistency,
    /// Confidence below threshold
    LowConfidence,
    /// Duplicate detection concerns
    DuplicationConcern,
}

/// Severity levels for quality issues
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum QualitySeverity {
    /// Minor issue, record can proceed
    Info,
    /// Notable issue worth flagging
    Warning,
    /// Significant issue requiring attention
    Error,
    /// Critical issue requiring quarantine
    Critical,
}

/// Trait for implementing Quality Gate assessment logic
pub trait QualityGate {
    /// Assess the quality of a normalized record
    fn assess(&self, record: &NormalizedRecord) -> anyhow::Result<QualityAssessedRecord>;
}

/// Default Quality Gate implementation with configurable rules
pub struct DefaultQualityGate {
    /// Configuration for quality assessment rules
    pub config: QualityGateConfig,
}

/// Configuration for Quality Gate assessment rules
#[derive(Debug, Clone)]
pub struct QualityGateConfig {
    /// Minimum confidence threshold for acceptance
    pub min_confidence: f64,
    /// Minimum quality score for acceptance
    pub min_quality_score: f64,
    /// Rule version identifier
    pub rule_version: String,
    /// Require venue coordinates
    pub require_venue_coordinates: bool,
    /// Require event dates in reasonable range
    pub require_valid_event_dates: bool,
    /// Days in future for event date validation
    pub max_future_days: i64,
    /// Days in past for event date validation  
    pub max_past_days: i64,
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            min_quality_score: 0.6,
            rule_version: "v1.0.0".to_string(),
            require_venue_coordinates: true,
            require_valid_event_dates: true,
            max_future_days: 365, // 1 year in future
            max_past_days: 30,    // 1 month in past
        }
    }
}

impl DefaultQualityGate {
    /// Create a new Quality Gate with default configuration
    pub fn new() -> Self {
        Self {
            config: QualityGateConfig::default(),
        }
    }

    /// Create a Quality Gate with custom configuration
    pub fn with_config(config: QualityGateConfig) -> Self {
        Self { config }
    }

    /// Assess entity-specific quality rules
    fn assess_entity_quality(&self, record: &NormalizedRecord) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        match &record.entity {
            crate::pipeline::processing::normalize::NormalizedEntity::Event(event) => {
                issues.extend(self.assess_event_quality(event));
            }
            crate::pipeline::processing::normalize::NormalizedEntity::Venue(venue) => {
                issues.extend(self.assess_venue_quality(venue));
            }
            crate::pipeline::processing::normalize::NormalizedEntity::Artist(artist) => {
                issues.extend(self.assess_artist_quality(artist));
            }
        }

        issues
    }

    /// Assess event-specific quality
    fn assess_event_quality(&self, event: &crate::domain::Event) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        // Check if title is meaningful
        if event.title.trim().is_empty() || event.title.len() < 3 {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::MissingData,
                severity: QualitySeverity::Error,
                description: "Event title is missing or too short".to_string(),
                field: Some("title".to_string()),
                suggestion: Some("Event title should be at least 3 characters".to_string()),
            });
        }

        // Check event date validity
        if self.config.require_valid_event_dates {
            let today = Utc::now().naive_utc().date();
            let days_diff = (event.event_day - today).num_days();

            if days_diff > self.config.max_future_days {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::OutOfRange,
                    severity: QualitySeverity::Warning,
                    description: format!("Event date is {} days in the future", days_diff),
                    field: Some("event_day".to_string()),
                    suggestion: Some("Verify event date is correct".to_string()),
                });
            } else if days_diff < -self.config.max_past_days {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::OutOfRange,
                    severity: QualitySeverity::Warning,
                    description: format!("Event date is {} days in the past", -days_diff),
                    field: Some("event_day".to_string()),
                    suggestion: Some("Verify this is not a historical event".to_string()),
                });
            }
        }

        // Check for placeholder venue_id
        if event.venue_id.is_nil() {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::MissingData,
                severity: QualitySeverity::Info,
                description: "Event has placeholder venue_id (will be resolved in conflation)".to_string(),
                field: Some("venue_id".to_string()),
                suggestion: None,
            });
        }

        issues
    }

    /// Assess venue-specific quality
    fn assess_venue_quality(&self, venue: &crate::domain::Venue) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        // Check venue name
        if venue.name.trim().is_empty() {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::MissingData,
                severity: QualitySeverity::Critical,
                description: "Venue name is missing".to_string(),
                field: Some("name".to_string()),
                suggestion: Some("Venue must have a valid name".to_string()),
            });
        }

        // Check coordinates if required
        if self.config.require_venue_coordinates {
            // Check for default Seattle coordinates (indicates geocoding placeholder)
            if (venue.latitude - 47.6062).abs() < 0.0001 && (venue.longitude + 122.3321).abs() < 0.0001 {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::IncompleteGeography,
                    severity: QualitySeverity::Warning,
                    description: "Venue using default Seattle coordinates".to_string(),
                    field: Some("coordinates".to_string()),
                    suggestion: Some("Provide specific venue address for accurate geocoding".to_string()),
                });
            }

            // Check for reasonable coordinate ranges (Seattle area)
            if venue.latitude < 47.0 || venue.latitude > 48.0 || venue.longitude < -123.0 || venue.longitude > -121.0 {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::OutOfRange,
                    severity: QualitySeverity::Error,
                    description: "Venue coordinates appear to be outside Seattle area".to_string(),
                    field: Some("coordinates".to_string()),
                    suggestion: Some("Verify coordinates are in Seattle region".to_string()),
                });
            }
        }

        // Check address completeness
        if venue.address.trim().is_empty() {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::MissingData,
                severity: QualitySeverity::Warning,
                description: "Venue address is missing".to_string(),
                field: Some("address".to_string()),
                suggestion: Some("Address helps with venue identification and geocoding".to_string()),
            });
        }

        issues
    }

    /// Assess artist-specific quality
    fn assess_artist_quality(&self, artist: &crate::domain::Artist) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        // Check artist name
        if artist.name.trim().is_empty() {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::MissingData,
                severity: QualitySeverity::Critical,
                description: "Artist name is missing".to_string(),
                field: Some("name".to_string()),
                suggestion: Some("Artist must have a valid name".to_string()),
            });
        } else if artist.name.len() < 2 {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::SuspiciousValue,
                severity: QualitySeverity::Warning,
                description: "Artist name is unusually short".to_string(),
                field: Some("name".to_string()),
                suggestion: Some("Verify artist name is complete".to_string()),
            });
        }

        issues
    }

    /// Calculate overall quality score based on issues
    fn calculate_quality_score(&self, issues: &[QualityIssue], confidence: f64) -> f64 {
        let mut score = confidence; // Start with normalization confidence

        // Deduct points based on issue severity
        for issue in issues {
            let deduction = match issue.severity {
                QualitySeverity::Info => 0.01,
                QualitySeverity::Warning => 0.05,
                QualitySeverity::Error => 0.15,
                QualitySeverity::Critical => 0.30,
            };
            score = (score - deduction).max(0.0);
        }

        score
    }

    /// Determine quality decision based on score and issues
    fn determine_decision(&self, quality_score: f64, issues: &[QualityIssue]) -> QualityDecision {
        // Check for critical issues first
        if issues.iter().any(|i| i.severity == QualitySeverity::Critical) {
            return QualityDecision::Quarantine;
        }

        // Check quality score threshold
        if quality_score < self.config.min_quality_score {
            return QualityDecision::Quarantine;
        }

        // Check for warnings
        if issues.iter().any(|i| i.severity >= QualitySeverity::Warning) {
            return QualityDecision::AcceptWithWarnings;
        }

        QualityDecision::Accept
    }
}

impl QualityGate for DefaultQualityGate {
    fn assess(&self, record: &NormalizedRecord) -> anyhow::Result<QualityAssessedRecord> {
        let mut issues = Vec::new();

        // Check normalization confidence
        if record.normalization.confidence < self.config.min_confidence {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::LowConfidence,
                severity: QualitySeverity::Warning,
                description: format!(
                    "Normalization confidence {:.2} below threshold {:.2}",
                    record.normalization.confidence, self.config.min_confidence
                ),
                field: Some("confidence".to_string()),
                suggestion: Some("Review source data quality".to_string()),
            });
        }

        // Check for normalization warnings
        for warning in &record.normalization.warnings {
            issues.push(QualityIssue {
                issue_type: QualityIssueType::SuspiciousValue,
                severity: QualitySeverity::Info,
                description: format!("Normalization warning: {}", warning),
                field: None,
                suggestion: None,
            });
        }

        // Assess entity-specific quality
        issues.extend(self.assess_entity_quality(record));

        // Calculate quality score and decision
        let quality_score = self.calculate_quality_score(&issues, record.normalization.confidence);
        let decision = self.determine_decision(quality_score, &issues);

        let assessment = QualityAssessment {
            decision,
            quality_score,
            issues,
            rule_version: self.config.rule_version.clone(),
        };

        Ok(QualityAssessedRecord {
            normalized_record: record.clone(),
            quality_assessment: assessment,
            assessed_at: Utc::now(),
        })
    }
}

impl Default for DefaultQualityGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Artist, Event, Venue};
    use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
    use chrono::{NaiveDate, Utc};
    use serde_json::json;
    use uuid::Uuid;

    fn create_test_event() -> NormalizedRecord {
        let event = Event {
            id: None,
            title: "Test Concert".to_string(),
            event_day: NaiveDate::from_ymd_opt(2025, 8, 20).unwrap(),
            start_time: None,
            event_url: None,
            description: None,
            event_image_url: None,
            venue_id: Uuid::nil(),
            artist_ids: Vec::new(),
            show_event: true,
            finalized: false,
            created_at: Utc::now(),
        };

        NormalizedRecord {
            entity: NormalizedEntity::Event(event),
            provenance: RecordProvenance {
                envelope_id: "test_envelope".to_string(),
                source_id: "test_source".to_string(),
                payload_ref: "test_payload".to_string(),
                record_path: "$.events[0]".to_string(),
                normalized_at: Utc::now(),
            },
            normalization: NormalizationMetadata {
                confidence: 0.8,
                warnings: Vec::new(),
                geocoded: false,
                strategy: "default".to_string(),
            },
        }
    }

    #[test]
    fn test_quality_gate_accepts_good_event() {
        let gate = DefaultQualityGate::new();
        let record = create_test_event();

        let result = gate.assess(&record).unwrap();
        assert_eq!(result.quality_assessment.decision, QualityDecision::Accept);
        assert!(result.quality_assessment.quality_score > 0.6);
    }

    #[test]
    fn test_quality_gate_flags_low_confidence() {
        let gate = DefaultQualityGate::new();
        let mut record = create_test_event();
        record.normalization.confidence = 0.5; // Below threshold

        let result = gate.assess(&record).unwrap();
        assert_eq!(result.quality_assessment.decision, QualityDecision::AcceptWithWarnings);
        
        let has_confidence_issue = result.quality_assessment.issues
            .iter()
            .any(|i| matches!(i.issue_type, QualityIssueType::LowConfidence));
        assert!(has_confidence_issue);
    }

    #[test]
    fn test_quality_gate_quarantines_missing_title() {
        let gate = DefaultQualityGate::new();
        let mut record = create_test_event();
        
        if let NormalizedEntity::Event(ref mut event) = record.entity {
            event.title = "".to_string(); // Empty title
        }

        let result = gate.assess(&record).unwrap();
        assert_eq!(result.quality_assessment.decision, QualityDecision::Quarantine);
        
        let has_missing_data_issue = result.quality_assessment.issues
            .iter()
            .any(|i| matches!(i.issue_type, QualityIssueType::MissingData));
        assert!(has_missing_data_issue);
    }
}
