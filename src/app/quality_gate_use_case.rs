use anyhow::Result;

use crate::app::ports::QualityGateOutputPort;
use crate::pipeline::processing::normalize::NormalizedRecord;
use crate::pipeline::processing::quality_gate::{DefaultQualityGate, QualityAssessedRecord, QualityGate, QualityDecision};

/// Use case for assessing quality of normalized records through the Quality Gate
pub struct QualityGateUseCase {
    quality_gate: Box<dyn QualityGate + Send + Sync>,
    accepted_output: Box<dyn QualityGateOutputPort>,
    quarantined_output: Box<dyn QualityGateOutputPort>,
}

impl QualityGateUseCase {
    pub fn new(
        quality_gate: Box<dyn QualityGate + Send + Sync>,
        accepted_output: Box<dyn QualityGateOutputPort>,
        quarantined_output: Box<dyn QualityGateOutputPort>,
    ) -> Self {
        Self {
            quality_gate,
            accepted_output,
            quarantined_output,
        }
    }

    /// Create a use case with the default quality gate
    pub fn with_default_quality_gate(
        accepted_output: Box<dyn QualityGateOutputPort>,
        quarantined_output: Box<dyn QualityGateOutputPort>,
    ) -> Self {
        Self {
            quality_gate: Box::new(DefaultQualityGate::new()),
            accepted_output,
            quarantined_output,
        }
    }

    /// Assess quality of a single normalized record
    pub async fn assess_record(&self, record: &NormalizedRecord) -> Result<QualityAssessedRecord> {
        // Apply quality assessment logic
        let assessed_record = self.quality_gate.assess(record)?;

        // Emit metrics for quality assessment
        match assessed_record.quality_assessment.decision {
            QualityDecision::Accept => {
                crate::observability::metrics::quality_gate::record_accepted();
            }
            QualityDecision::AcceptWithWarnings => {
                crate::observability::metrics::quality_gate::record_accepted_with_warnings();
            }
            QualityDecision::Quarantine => {
                crate::observability::metrics::quality_gate::record_quarantined();
            }
        }

        // Record quality score
        crate::observability::metrics::quality_gate::quality_score_recorded(
            assessed_record.quality_assessment.quality_score
        );

        // Record issues by type and severity
        for issue in &assessed_record.quality_assessment.issues {
            crate::observability::metrics::quality_gate::issue_detected(
                &format!("{:?}", issue.issue_type),
                &format!("{:?}", issue.severity)
            );
        }

        // Route to appropriate output based on decision
        match assessed_record.quality_assessment.decision {
            QualityDecision::Accept | QualityDecision::AcceptWithWarnings => {
                self.accepted_output.write_quality_assessed_record(&assessed_record).await?;
            }
            QualityDecision::Quarantine => {
                self.quarantined_output.write_quality_assessed_record(&assessed_record).await?;
            }
        }

        Ok(assessed_record)
    }

    /// Assess quality of multiple normalized records in batch
    pub async fn assess_batch(&self, records: &[NormalizedRecord]) -> Result<Vec<QualityAssessedRecord>> {
        let mut all_assessed = Vec::new();
        let mut accepted_count = 0;
        let mut quarantined_count = 0;

        for record in records {
            let assessed = self.assess_record(record).await?;
            
            match assessed.quality_assessment.decision {
                QualityDecision::Accept | QualityDecision::AcceptWithWarnings => {
                    accepted_count += 1;
                }
                QualityDecision::Quarantine => {
                    quarantined_count += 1;
                }
            }
            
            all_assessed.push(assessed);
        }

        // Record batch metrics
        crate::observability::metrics::quality_gate::batch_processed(records.len(), accepted_count, quarantined_count);
        
        Ok(all_assessed)
    }

    /// Get statistics for the current batch assessment
    pub fn get_batch_stats(assessed_records: &[QualityAssessedRecord]) -> QualityGateBatchStats {
        let mut stats = QualityGateBatchStats::default();
        
        for record in assessed_records {
            stats.total_records += 1;
            
            match record.quality_assessment.decision {
                QualityDecision::Accept => stats.accepted_count += 1,
                QualityDecision::AcceptWithWarnings => stats.accepted_with_warnings_count += 1,
                QualityDecision::Quarantine => stats.quarantined_count += 1,
            }
            
            // Update quality score stats
            let score = record.quality_assessment.quality_score;
            if stats.min_quality_score.is_none() || score < stats.min_quality_score.unwrap() {
                stats.min_quality_score = Some(score);
            }
            if stats.max_quality_score.is_none() || score > stats.max_quality_score.unwrap() {
                stats.max_quality_score = Some(score);
            }
            stats.avg_quality_score = if stats.total_records == 1 {
                score
            } else {
                (stats.avg_quality_score * (stats.total_records - 1) as f64 + score) / stats.total_records as f64
            };
            
            // Count issues by severity
            for issue in &record.quality_assessment.issues {
                match issue.severity {
                    crate::pipeline::processing::quality_gate::QualitySeverity::Info => stats.info_issues += 1,
                    crate::pipeline::processing::quality_gate::QualitySeverity::Warning => stats.warning_issues += 1,
                    crate::pipeline::processing::quality_gate::QualitySeverity::Error => stats.error_issues += 1,
                    crate::pipeline::processing::quality_gate::QualitySeverity::Critical => stats.critical_issues += 1,
                }
            }
        }
        
        stats
    }
}

/// Statistics for a batch of quality gate assessments
#[derive(Debug, Default)]
pub struct QualityGateBatchStats {
    pub total_records: usize,
    pub accepted_count: usize,
    pub accepted_with_warnings_count: usize,
    pub quarantined_count: usize,
    pub min_quality_score: Option<f64>,
    pub max_quality_score: Option<f64>,
    pub avg_quality_score: f64,
    pub info_issues: usize,
    pub warning_issues: usize,
    pub error_issues: usize,
    pub critical_issues: usize,
}

impl QualityGateBatchStats {
    /// Calculate acceptance rate as percentage
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        (self.accepted_count + self.accepted_with_warnings_count) as f64 / self.total_records as f64 * 100.0
    }

    /// Calculate quarantine rate as percentage
    pub fn quarantine_rate(&self) -> f64 {
        if self.total_records == 0 {
            return 0.0;
        }
        self.quarantined_count as f64 / self.total_records as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::QualityGateOutputPort;
    use crate::pipeline::processing::quality_gate::QualityAssessedRecord;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockQualityGateOutput {
        pub records: Arc<Mutex<Vec<QualityAssessedRecord>>>,
    }

    impl MockQualityGateOutput {
        pub fn new() -> Self {
            Self {
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait]
    impl QualityGateOutputPort for MockQualityGateOutput {
        async fn write_quality_assessed_record(&self, record: &QualityAssessedRecord) -> Result<()> {
            self.records.lock().await.push(record.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_quality_gate_use_case() {
        let accepted_output = Box::new(MockQualityGateOutput::new());
        let quarantined_output = Box::new(MockQualityGateOutput::new());
        
        let accepted_records = accepted_output.records.clone();
        let quarantined_records = quarantined_output.records.clone();
        
        let use_case = QualityGateUseCase::with_default_quality_gate(accepted_output, quarantined_output);

        // Create a test normalized record (using a helper from quality_gate tests)
        use crate::domain::Event;
        use crate::pipeline::processing::normalize::{NormalizedEntity, NormalizedRecord, NormalizationMetadata, RecordProvenance};
        use chrono::{NaiveDate, Utc};
        use uuid::Uuid;

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

        let normalized_record = NormalizedRecord {
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
        };

        let result = use_case.assess_record(&normalized_record).await;
        assert!(result.is_ok());

        // Should be accepted (good quality record)
        let accepted = accepted_records.lock().await;
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].quality_assessment.decision, QualityDecision::Accept);

        let quarantined = quarantined_records.lock().await;
        assert_eq!(quarantined.len(), 0);
    }
}
