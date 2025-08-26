//! Simple metrics module for the SMS scraper system
//! 
//! This module provides a straightforward API for recording metrics using
//! the standard Prometheus naming conventions.

pub mod dashboard;

use std::fmt;

/// Enum representing all metric names used in the system
/// This eliminates magic strings and provides compile-time safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricName {
    // Heartbeat
    Heartbeat,
    
    // Sources metrics
    SourcesRequestsSuccess,
    SourcesRequestsError,
    SourcesRequestDuration,
    SourcesPayloadBytes,
    SourcesRegistryLoadsSuccess,
    SourcesRegistryLoadsError,
    
    // Gateway metrics
    GatewayEnvelopesAccepted,
    GatewayEnvelopesDeduplicated,
    GatewayCasWritesSuccess,
    GatewayCasWritesError,
    GatewayRecordsIngested,
    GatewayProcessingDuration,
    GatewayIngestSuccess,
    GatewayIngestError,
    GatewayBytesIngested,
    GatewayIngestDuration,
    GatewayEnvelopeCreated,
    
    // Ingest log metrics
    IngestLogWritesSuccess,
    IngestLogWritesError,
    IngestLogWriteBytes,
    IngestLogRotations,
    IngestLogCurrentFileBytes,
    IngestLogActiveConsumers,
    
    // Parser metrics
    ParserParseSuccess,
    ParserParseError,
    ParserDuration,
    ParserRecordsExtracted,
    ParserBytesProcessed,
    ParserBatchSize,
    
    // Normalize metrics
    NormalizeRecordsProcessed,
    NormalizeConfidence,
    NormalizeGeocoding,
    NormalizeWarnings,
    NormalizeBatchesProcessed,
    NormalizeBatchSize,
    
    // Quality Gate metrics
    QualityGateRecordsAccepted,
    QualityGateRecordsAcceptedWithWarnings,
    QualityGateRecordsQuarantined,
    QualityGateQualityScore,
    QualityGateIssuesDetected,
    QualityGateBatchesProcessed,
    QualityGateBatchSize,
    
    // Enrich metrics
    EnrichRecordsProcessed,
    EnrichConfidence,
    EnrichSpatialBinning,
    EnrichCityTagging,
    EnrichTagsAdded,
    EnrichWarnings,
    EnrichBatchesProcessed,
    EnrichBatchSize,
    
    // Conflation metrics
    ConflationRecordsProcessed,
    ConflationRecordsSuccessful,
    ConflationRecordsFailed,
    ConflationConfidenceScore,
    ConflationNewEntities,
    ConflationMatchedExisting,
    ConflationUpdatedExisting,
    ConflationDuplicates,
    ConflationUncertainResolutions,
    ConflationWarnings,
    ConflationPotentialDuplicates,
    ConflationAlternativeMatches,
    ConflationBatchesProcessed,
    ConflationBatchesSuccessful,
    ConflationBatchSize,
    ConflationBatchProcessingDuration,
    ConflationBatchRecordsSuccessful,
    ConflationBatchRecordsFailed,
    
}

impl fmt::Display for MetricName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            // Heartbeat
            MetricName::Heartbeat => "sms_heartbeat_total",
            
            // Sources metrics
            MetricName::SourcesRequestsSuccess => "sms_sources_requests_success_total",
            MetricName::SourcesRequestsError => "sms_sources_requests_error_total",
            MetricName::SourcesRequestDuration => "sms_sources_request_duration_seconds",
            MetricName::SourcesPayloadBytes => "sms_sources_payload_bytes",
            MetricName::SourcesRegistryLoadsSuccess => "sms_sources_registry_loads_success_total",
            MetricName::SourcesRegistryLoadsError => "sms_sources_registry_loads_error_total",
            
            // Gateway metrics
            MetricName::GatewayEnvelopesAccepted => "sms_gateway_envelopes_accepted_total",
            MetricName::GatewayEnvelopesDeduplicated => "sms_gateway_envelopes_deduplicated_total",
            MetricName::GatewayCasWritesSuccess => "sms_gateway_cas_writes_success_total",
            MetricName::GatewayCasWritesError => "sms_gateway_cas_writes_error_total",
            MetricName::GatewayRecordsIngested => "sms_gateway_records_ingested_total",
            MetricName::GatewayProcessingDuration => "sms_gateway_processing_duration_seconds",
            MetricName::GatewayIngestSuccess => "sms_gateway_ingest_success_total",
            MetricName::GatewayIngestError => "sms_gateway_ingest_error_total",
            MetricName::GatewayBytesIngested => "sms_gateway_bytes_ingested",
            MetricName::GatewayIngestDuration => "sms_gateway_ingest_duration_seconds",
            MetricName::GatewayEnvelopeCreated => "sms_gateway_envelope_created",
            
            // Ingest log metrics
            MetricName::IngestLogWritesSuccess => "sms_ingest_log_writes_success_total",
            MetricName::IngestLogWritesError => "sms_ingest_log_writes_error_total",
            MetricName::IngestLogWriteBytes => "sms_ingest_log_write_bytes",
            MetricName::IngestLogRotations => "sms_ingest_log_rotations_total",
            MetricName::IngestLogCurrentFileBytes => "sms_ingest_log_current_file_bytes",
            MetricName::IngestLogActiveConsumers => "sms_ingest_log_active_consumers",
            
            // Parser metrics
            MetricName::ParserParseSuccess => "sms_parser_parse_success_total",
            MetricName::ParserParseError => "sms_parser_parse_error_total",
            MetricName::ParserDuration => "sms_parser_duration_seconds",
            MetricName::ParserRecordsExtracted => "sms_parser_records_extracted_total",
            MetricName::ParserBytesProcessed => "sms_parser_bytes_processed",
            MetricName::ParserBatchSize => "sms_parser_batch_size",
            
            // Normalize metrics
            MetricName::NormalizeRecordsProcessed => "sms_normalize_records_processed_total",
            MetricName::NormalizeConfidence => "sms_normalize_confidence",
            MetricName::NormalizeGeocoding => "sms_normalize_geocoding_total",
            MetricName::NormalizeWarnings => "sms_normalize_warnings_total",
            MetricName::NormalizeBatchesProcessed => "sms_normalize_batches_processed_total",
            MetricName::NormalizeBatchSize => "sms_normalize_batch_size",
            
            // Quality Gate metrics
            MetricName::QualityGateRecordsAccepted => "sms_quality_gate_records_accepted_total",
            MetricName::QualityGateRecordsAcceptedWithWarnings => "sms_quality_gate_records_accepted_with_warnings_total",
            MetricName::QualityGateRecordsQuarantined => "sms_quality_gate_records_quarantined_total",
            MetricName::QualityGateQualityScore => "sms_quality_gate_quality_score",
            MetricName::QualityGateIssuesDetected => "sms_quality_gate_issues_detected_total",
            MetricName::QualityGateBatchesProcessed => "sms_quality_gate_batches_processed_total",
            MetricName::QualityGateBatchSize => "sms_quality_gate_batch_size",
            
            // Enrich metrics
            MetricName::EnrichRecordsProcessed => "sms_enrich_records_processed_total",
            MetricName::EnrichConfidence => "sms_enrich_confidence",
            MetricName::EnrichSpatialBinning => "sms_enrich_spatial_binning_total",
            MetricName::EnrichCityTagging => "sms_enrich_city_tagging_total",
            MetricName::EnrichTagsAdded => "sms_enrich_tags_added",
            MetricName::EnrichWarnings => "sms_enrich_warnings_total",
            MetricName::EnrichBatchesProcessed => "sms_enrich_batches_processed_total",
            MetricName::EnrichBatchSize => "sms_enrich_batch_size",
            
            // Conflation metrics
            MetricName::ConflationRecordsProcessed => "sms_conflation_records_processed_total",
            MetricName::ConflationRecordsSuccessful => "sms_conflation_records_successful_total",
            MetricName::ConflationRecordsFailed => "sms_conflation_records_failed_total",
            MetricName::ConflationConfidenceScore => "sms_conflation_confidence_score",
            MetricName::ConflationNewEntities => "sms_conflation_new_entities_total",
            MetricName::ConflationMatchedExisting => "sms_conflation_matched_existing_total",
            MetricName::ConflationUpdatedExisting => "sms_conflation_updated_existing_total",
            MetricName::ConflationDuplicates => "sms_conflation_duplicates_total",
            MetricName::ConflationUncertainResolutions => "sms_conflation_uncertain_resolutions_total",
            MetricName::ConflationWarnings => "sms_conflation_warnings_total",
            MetricName::ConflationPotentialDuplicates => "sms_conflation_potential_duplicates_total",
            MetricName::ConflationAlternativeMatches => "sms_conflation_alternative_matches_total",
            MetricName::ConflationBatchesProcessed => "sms_conflation_batches_processed_total",
            MetricName::ConflationBatchesSuccessful => "sms_conflation_batches_successful_total",
            MetricName::ConflationBatchSize => "sms_conflation_batch_size",
            MetricName::ConflationBatchProcessingDuration => "sms_conflation_batch_processing_duration_seconds",
            MetricName::ConflationBatchRecordsSuccessful => "sms_conflation_batch_records_successful_total",
            MetricName::ConflationBatchRecordsFailed => "sms_conflation_batch_records_failed_total",
            
        };
        write!(f, "{}", name)
    }
}

impl MetricName {
    /// Get the metric name as a string (convenience method)
    pub fn as_str(&self) -> &'static str {
        match self {
            // Heartbeat
            MetricName::Heartbeat => "sms_heartbeat_total",
            
            // Sources metrics
            MetricName::SourcesRequestsSuccess => "sms_sources_requests_success_total",
            MetricName::SourcesRequestsError => "sms_sources_requests_error_total",
            MetricName::SourcesRequestDuration => "sms_sources_request_duration_seconds",
            MetricName::SourcesPayloadBytes => "sms_sources_payload_bytes",
            MetricName::SourcesRegistryLoadsSuccess => "sms_sources_registry_loads_success_total",
            MetricName::SourcesRegistryLoadsError => "sms_sources_registry_loads_error_total",
            
            // Gateway metrics
            MetricName::GatewayEnvelopesAccepted => "sms_gateway_envelopes_accepted_total",
            MetricName::GatewayEnvelopesDeduplicated => "sms_gateway_envelopes_deduplicated_total",
            MetricName::GatewayCasWritesSuccess => "sms_gateway_cas_writes_success_total",
            MetricName::GatewayCasWritesError => "sms_gateway_cas_writes_error_total",
            MetricName::GatewayRecordsIngested => "sms_gateway_records_ingested_total",
            MetricName::GatewayProcessingDuration => "sms_gateway_processing_duration_seconds",
            MetricName::GatewayIngestSuccess => "sms_gateway_ingest_success_total",
            MetricName::GatewayIngestError => "sms_gateway_ingest_error_total",
            MetricName::GatewayBytesIngested => "sms_gateway_bytes_ingested",
            MetricName::GatewayIngestDuration => "sms_gateway_ingest_duration_seconds",
            MetricName::GatewayEnvelopeCreated => "sms_gateway_envelope_created",
            
            // Ingest log metrics
            MetricName::IngestLogWritesSuccess => "sms_ingest_log_writes_success_total",
            MetricName::IngestLogWritesError => "sms_ingest_log_writes_error_total",
            MetricName::IngestLogWriteBytes => "sms_ingest_log_write_bytes",
            MetricName::IngestLogRotations => "sms_ingest_log_rotations_total",
            MetricName::IngestLogCurrentFileBytes => "sms_ingest_log_current_file_bytes",
            MetricName::IngestLogActiveConsumers => "sms_ingest_log_active_consumers",
            
            // Parser metrics
            MetricName::ParserParseSuccess => "sms_parser_parse_success_total",
            MetricName::ParserParseError => "sms_parser_parse_error_total",
            MetricName::ParserDuration => "sms_parser_duration_seconds",
            MetricName::ParserRecordsExtracted => "sms_parser_records_extracted_total",
            MetricName::ParserBytesProcessed => "sms_parser_bytes_processed",
            MetricName::ParserBatchSize => "sms_parser_batch_size",
            
            // Normalize metrics
            MetricName::NormalizeRecordsProcessed => "sms_normalize_records_processed_total",
            MetricName::NormalizeConfidence => "sms_normalize_confidence",
            MetricName::NormalizeGeocoding => "sms_normalize_geocoding_total",
            MetricName::NormalizeWarnings => "sms_normalize_warnings_total",
            MetricName::NormalizeBatchesProcessed => "sms_normalize_batches_processed_total",
            MetricName::NormalizeBatchSize => "sms_normalize_batch_size",
            
            // Quality Gate metrics
            MetricName::QualityGateRecordsAccepted => "sms_quality_gate_records_accepted_total",
            MetricName::QualityGateRecordsAcceptedWithWarnings => "sms_quality_gate_records_accepted_with_warnings_total",
            MetricName::QualityGateRecordsQuarantined => "sms_quality_gate_records_quarantined_total",
            MetricName::QualityGateQualityScore => "sms_quality_gate_quality_score",
            MetricName::QualityGateIssuesDetected => "sms_quality_gate_issues_detected_total",
            MetricName::QualityGateBatchesProcessed => "sms_quality_gate_batches_processed_total",
            MetricName::QualityGateBatchSize => "sms_quality_gate_batch_size",
            
            // Enrich metrics
            MetricName::EnrichRecordsProcessed => "sms_enrich_records_processed_total",
            MetricName::EnrichConfidence => "sms_enrich_confidence",
            MetricName::EnrichSpatialBinning => "sms_enrich_spatial_binning_total",
            MetricName::EnrichCityTagging => "sms_enrich_city_tagging_total",
            MetricName::EnrichTagsAdded => "sms_enrich_tags_added",
            MetricName::EnrichWarnings => "sms_enrich_warnings_total",
            MetricName::EnrichBatchesProcessed => "sms_enrich_batches_processed_total",
            MetricName::EnrichBatchSize => "sms_enrich_batch_size",
            
            // Conflation metrics
            MetricName::ConflationRecordsProcessed => "sms_conflation_records_processed_total",
            MetricName::ConflationRecordsSuccessful => "sms_conflation_records_successful_total",
            MetricName::ConflationRecordsFailed => "sms_conflation_records_failed_total",
            MetricName::ConflationConfidenceScore => "sms_conflation_confidence_score",
            MetricName::ConflationNewEntities => "sms_conflation_new_entities_total",
            MetricName::ConflationMatchedExisting => "sms_conflation_matched_existing_total",
            MetricName::ConflationUpdatedExisting => "sms_conflation_updated_existing_total",
            MetricName::ConflationDuplicates => "sms_conflation_duplicates_total",
            MetricName::ConflationUncertainResolutions => "sms_conflation_uncertain_resolutions_total",
            MetricName::ConflationWarnings => "sms_conflation_warnings_total",
            MetricName::ConflationPotentialDuplicates => "sms_conflation_potential_duplicates_total",
            MetricName::ConflationAlternativeMatches => "sms_conflation_alternative_matches_total",
            MetricName::ConflationBatchesProcessed => "sms_conflation_batches_processed_total",
            MetricName::ConflationBatchesSuccessful => "sms_conflation_batches_successful_total",
            MetricName::ConflationBatchSize => "sms_conflation_batch_size",
            MetricName::ConflationBatchProcessingDuration => "sms_conflation_batch_processing_duration_seconds",
            MetricName::ConflationBatchRecordsSuccessful => "sms_conflation_batch_records_successful_total",
            MetricName::ConflationBatchRecordsFailed => "sms_conflation_batch_records_failed_total",
            
        }
    }


    /// Get all metric names as an iterator (for dynamic dashboard generation)
    pub fn all_metrics() -> impl Iterator<Item = MetricName> {
        use MetricName::*;
        [
            // Heartbeat
            Heartbeat,
            
            // Sources metrics
            SourcesRequestsSuccess,
            SourcesRequestsError,
            SourcesRequestDuration,
            SourcesPayloadBytes,
            SourcesRegistryLoadsSuccess,
            SourcesRegistryLoadsError,
            
            // Gateway metrics
            GatewayEnvelopesAccepted,
            GatewayEnvelopesDeduplicated,
            GatewayCasWritesSuccess,
            GatewayCasWritesError,
            GatewayRecordsIngested,
            GatewayProcessingDuration,
            GatewayIngestSuccess,
            GatewayIngestError,
            GatewayBytesIngested,
            GatewayIngestDuration,
            GatewayEnvelopeCreated,
            
            // Ingest log metrics
            IngestLogWritesSuccess,
            IngestLogWritesError,
            IngestLogWriteBytes,
            IngestLogRotations,
            IngestLogCurrentFileBytes,
            IngestLogActiveConsumers,
            
            // Parser metrics
            ParserParseSuccess,
            ParserParseError,
            ParserDuration,
            ParserRecordsExtracted,
            ParserBytesProcessed,
            ParserBatchSize,
            
            // Normalize metrics
            NormalizeRecordsProcessed,
            NormalizeConfidence,
            NormalizeGeocoding,
            NormalizeWarnings,
            NormalizeBatchesProcessed,
            NormalizeBatchSize,

            // Quality Gate metrics
            QualityGateRecordsAccepted,
            QualityGateRecordsAcceptedWithWarnings,
            QualityGateRecordsQuarantined,
            QualityGateQualityScore,
            QualityGateIssuesDetected,
            QualityGateBatchesProcessed,
            QualityGateBatchSize,
            
            // Enrich metrics
            EnrichRecordsProcessed,
            EnrichConfidence,
            EnrichSpatialBinning,
            EnrichCityTagging,
            EnrichTagsAdded,
            EnrichWarnings,
            EnrichBatchesProcessed,
            EnrichBatchSize,
            
            // Push gateway metrics (usually not displayed)
            // IngestTimestamp,
            // IngestBytes,
            // IngestDurationSeconds,
            // IngestSuccess,
            // PushTimestamp,
            // MetricsInitialized,
        ].into_iter()
    }

    /// Get metric metadata for dashboard generation
    pub fn metadata(&self) -> (&'static str, &'static str, Option<&'static str>) {
        // Returns (phase, description, unit)
        match self {
            // Heartbeat
            MetricName::Heartbeat => ("system", "Heartbeat counter", None),
            
            // Sources metrics
            MetricName::SourcesRequestsSuccess => ("sources", "Total successful source requests", None),
            MetricName::SourcesRequestsError => ("sources", "Total failed source requests", None),
            MetricName::SourcesRequestDuration => ("sources", "Request duration in seconds", Some("s")),
            MetricName::SourcesPayloadBytes => ("sources", "Payload size in bytes", Some("bytes")),
            MetricName::SourcesRegistryLoadsSuccess => ("sources", "Successful registry loads", None),
            MetricName::SourcesRegistryLoadsError => ("sources", "Failed registry loads", None),
            
            // Gateway metrics
            MetricName::GatewayEnvelopesAccepted => ("gateway", "Total envelopes accepted", None),
            MetricName::GatewayEnvelopesDeduplicated => ("gateway", "Total envelopes deduplicated", None),
            MetricName::GatewayCasWritesSuccess => ("gateway", "Successful CAS writes", None),
            MetricName::GatewayCasWritesError => ("gateway", "Failed CAS writes", None),
            MetricName::GatewayRecordsIngested => ("gateway", "Total records ingested", None),
            MetricName::GatewayProcessingDuration => ("gateway", "Gateway processing duration", Some("s")),
            MetricName::GatewayIngestSuccess => ("gateway", "Successful ingests by source", None),
            MetricName::GatewayIngestError => ("gateway", "Failed ingests by source", None),
            MetricName::GatewayBytesIngested => ("gateway", "Bytes ingested per source", Some("bytes")),
            MetricName::GatewayIngestDuration => ("gateway", "Ingest duration by source", Some("s")),
            MetricName::GatewayEnvelopeCreated => ("gateway", "Envelopes created", None),
            
            // Ingest log metrics
            MetricName::IngestLogWritesSuccess => ("ingest_log", "Successful log writes", None),
            MetricName::IngestLogWritesError => ("ingest_log", "Failed log writes", None),
            MetricName::IngestLogWriteBytes => ("ingest_log", "Log write size", Some("bytes")),
            MetricName::IngestLogRotations => ("ingest_log", "Log rotations", None),
            MetricName::IngestLogCurrentFileBytes => ("ingest_log", "Current log file size", Some("bytes")),
            MetricName::IngestLogActiveConsumers => ("ingest_log", "Active log consumers", None),
            
            // Parser metrics
            MetricName::ParserParseSuccess => ("parser", "Successful parses", None),
            MetricName::ParserParseError => ("parser", "Parse errors", None),
            MetricName::ParserDuration => ("parser", "Parse duration", Some("s")),
            MetricName::ParserRecordsExtracted => ("parser", "Records extracted", None),
            MetricName::ParserBytesProcessed => ("parser", "Bytes processed", Some("bytes")),
            MetricName::ParserBatchSize => ("parser", "Parse batch size", None),
            
            // Normalize metrics
            MetricName::NormalizeRecordsProcessed => ("normalize", "Records processed with normalization", None),
            MetricName::NormalizeConfidence => ("normalize", "Normalization confidence level", None),
            MetricName::NormalizeGeocoding => ("normalize", "Geocoding operations performed", None),
            MetricName::NormalizeWarnings => ("normalize", "Normalization warnings", None),
            MetricName::NormalizeBatchesProcessed => ("normalize", "Batches processed", None),
            MetricName::NormalizeBatchSize => ("normalize", "Normalization batch size", None),
            
            // Quality Gate metrics
            MetricName::QualityGateRecordsAccepted => ("quality_gate", "Records accepted by quality gate", None),
            MetricName::QualityGateRecordsAcceptedWithWarnings => ("quality_gate", "Records accepted with warnings", None),
            MetricName::QualityGateRecordsQuarantined => ("quality_gate", "Records quarantined by quality gate", None),
            MetricName::QualityGateQualityScore => ("quality_gate", "Quality score distribution", None),
            MetricName::QualityGateIssuesDetected => ("quality_gate", "Quality issues detected", None),
            MetricName::QualityGateBatchesProcessed => ("quality_gate", "Batches processed through quality gate", None),
            MetricName::QualityGateBatchSize => ("quality_gate", "Quality gate batch size", None),
            
            // Enrich metrics
            MetricName::EnrichRecordsProcessed => ("enrich", "Records processed with enrichment", None),
            MetricName::EnrichConfidence => ("enrich", "Enrichment confidence level", None),
            MetricName::EnrichSpatialBinning => ("enrich", "Spatial binning operations performed", None),
            MetricName::EnrichCityTagging => ("enrich", "City tagging operations performed", None),
            MetricName::EnrichTagsAdded => ("enrich", "Tags added per record", None),
            MetricName::EnrichWarnings => ("enrich", "Enrichment warnings", None),
            MetricName::EnrichBatchesProcessed => ("enrich", "Batches processed through enrichment", None),
            MetricName::EnrichBatchSize => ("enrich", "Enrichment batch size", None),
            
            // Conflation metrics
            MetricName::ConflationRecordsProcessed => ("conflation", "Records processed through conflation", None),
            MetricName::ConflationRecordsSuccessful => ("conflation", "Records successfully conflated", None),
            MetricName::ConflationRecordsFailed => ("conflation", "Records failed during conflation", None),
            MetricName::ConflationConfidenceScore => ("conflation", "Entity resolution confidence score", None),
            MetricName::ConflationNewEntities => ("conflation", "New canonical entities created", None),
            MetricName::ConflationMatchedExisting => ("conflation", "Records matched to existing entities", None),
            MetricName::ConflationUpdatedExisting => ("conflation", "Existing entities updated", None),
            MetricName::ConflationDuplicates => ("conflation", "Duplicate records identified", None),
            MetricName::ConflationUncertainResolutions => ("conflation", "Uncertain resolution decisions", None),
            MetricName::ConflationWarnings => ("conflation", "Conflation warnings generated", None),
            MetricName::ConflationPotentialDuplicates => ("conflation", "Potential duplicates identified", None),
            MetricName::ConflationAlternativeMatches => ("conflation", "Alternative matches considered", None),
            MetricName::ConflationBatchesProcessed => ("conflation", "Batches processed through conflation", None),
            MetricName::ConflationBatchesSuccessful => ("conflation", "Successful conflation batches", None),
            MetricName::ConflationBatchSize => ("conflation", "Conflation batch size", None),
            MetricName::ConflationBatchProcessingDuration => ("conflation", "Batch processing duration", Some("s")),
            MetricName::ConflationBatchRecordsSuccessful => ("conflation", "Records successfully processed in batch", None),
            MetricName::ConflationBatchRecordsFailed => ("conflation", "Records failed in batch processing", None),
            
        }
    }

    /// Infer metric type from metric name patterns
    pub fn infer_metric_type(&self) -> crate::observability::metrics::dashboard::MetricType {
        use crate::observability::metrics::dashboard::MetricType;
        let name = self.as_str();
        
        if name.contains("_total") || name.contains("success") || name.contains("error") || name.contains("extracted") || name.contains("accepted") || name.contains("quarantined") || name.contains("detected") || name.contains("processed") {
            MetricType::Counter
        } else if name.contains("_seconds") || name.contains("_bytes") || name.contains("_duration") || name.contains("_size") || name.contains("confidence") || name.contains("score") {
            MetricType::Histogram
        } else if name.contains("current_") || name.contains("active_") || name.contains("initialized") {
            MetricType::Gauge
        } else {
            // Default to counter for unknown patterns
            MetricType::Counter
        }
    }
}

use tracing::{info, warn};
use std::sync::Arc;

/// Initialize the metrics system with optional push gateway support
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    init_with_push_options(None, None)
}

/// Initialize with push gateway configuration
pub fn init_with_push_options(
    job_name: Option<&str>,
    instance: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    
    // Install the recorder and get the handle
    let handle = builder
        .install_recorder()
        .map_err(|e| format!("Failed to install Prometheus recorder: {}", e))?;
    
    // If push gateway is configured, store the handle for later pushing
    if let Ok(pushgateway_url) = std::env::var("SMS_PUSHGATEWAY_URL") {
        let job = job_name.unwrap_or("sms_scraper");
        let inst = instance.unwrap_or("default");
        
        // Store handle for push_all_metrics function
        METRICS_HANDLE.set(Arc::new(MetricsState {
            handle,
            pushgateway_url,
            job: job.to_string(),
            instance: inst.to_string(),
        })).ok();
        
        info!("Metrics system initialized with push gateway support");
    } else {
        info!("Metrics system initialized (no push gateway)");
    }
    
    Ok(())
}

// Global state for metrics pushing
use std::sync::OnceLock;
static METRICS_HANDLE: OnceLock<Arc<MetricsState>> = OnceLock::new();

/// Get access to the metrics handle for rendering
#[allow(dead_code)]
pub fn get_metrics_handle() -> Option<String> {
    METRICS_HANDLE.get().map(|state| state.handle.render())
}

struct MetricsState {
    handle: metrics_exporter_prometheus::PrometheusHandle,
    pushgateway_url: String,
    job: String,
    instance: String,
}

/// Internal function to push a single metric immediately
async fn push_single_metric(name: &str, value: f64, metric_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(state) = METRICS_HANDLE.get() {
        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            state.pushgateway_url.trim_end_matches('/'),
            state.job,
            state.instance
        );
        
        let metrics_text = format!(
            "# TYPE {} {}\n{} {}\n",
            name, metric_type, name, value
        );
        
        let client = reqwest::Client::new();
        let _ = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;
    }
    Ok(())
}

/// Internal function to push histogram metrics with buckets
async fn push_histogram_metric(name: &str, value: f64) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(state) = METRICS_HANDLE.get() {
        let push_url = format!(
            "{}/metrics/job/{}/instance/{}",
            state.pushgateway_url.trim_end_matches('/'),
            state.job,
            state.instance
        );
        
        // Define standard bucket boundaries (in bytes for payload size)
        let buckets = vec![
            1_000.0,      // 1KB
            10_000.0,     // 10KB
            100_000.0,    // 100KB
            1_000_000.0,  // 1MB
            10_000_000.0, // 10MB
            f64::INFINITY,
        ];
        
        // Build histogram metric text with buckets
        let mut metrics_text = format!("# TYPE {} histogram\n", name);
        
        // Add bucket entries
        let mut cumulative_count = 0u64;
        for bucket_bound in &buckets {
            if value <= *bucket_bound {
                cumulative_count = 1;
            }
            let le_value = if *bucket_bound == f64::INFINITY { 
                "+Inf".to_string() 
            } else { 
                bucket_bound.to_string() 
            };
            metrics_text.push_str(&format!(
                "{}_bucket{{le=\"{}\"}} {}\n",
                name,
                le_value,
                cumulative_count
            ));
        }
        
        // Add sum and count
        metrics_text.push_str(&format!(
            "{}_sum {}\n{}_count 1\n",
            name, value, name
        ));
        
        let client = reqwest::Client::new();
        let _ = client
            .post(&push_url)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(metrics_text)
            .send()
            .await?;
    }
    Ok(())
}

// Macro to automatically push metrics
macro_rules! counter_and_push {
    ($name:expr) => {{
        ::metrics::counter!($name).increment(1);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, 1.0, "counter").await;
        });
    }};
    ($name:expr, $($label_key:expr => $label_value:expr),+) => {{
        ::metrics::counter!($name, $($label_key => $label_value),+).increment(1);
        let name = $name.to_string();
        tokio::spawn(async move {
            let _ = push_single_metric(&name, 1.0, "counter").await;
        });
    }};
}

// Removed unused macros gauge_and_push and histogram_and_push

/// Record a heartbeat for testing
pub fn heartbeat() {
    let metric_name = MetricName::Heartbeat.as_str();
    ::metrics::counter!(metric_name).increment(1);
    tokio::spawn(async move {
        let _ = push_single_metric(metric_name, 1.0, "counter").await;
    });
}


// ============================================================================
// Sources Metrics
// ============================================================================

pub mod sources {
    use super::{push_single_metric, push_histogram_metric, MetricName};
    
    /// Record a successful request
    pub fn request_success() {
        let metric_name = MetricName::SourcesRequestsSuccess.as_str();
        ::metrics::counter!(metric_name).increment(1);
        // Immediately push this metric
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record a failed request
    pub fn request_error() {
        let metric_name = MetricName::SourcesRequestsError.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record request duration
    pub fn request_duration(secs: f64) {
        let metric_name = MetricName::SourcesRequestDuration.as_str();
        ::metrics::histogram!(metric_name).record(secs);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, secs, "gauge").await;
        });
    }
    
    /// Record payload size
    pub fn payload_bytes(bytes: usize) {
        let b = bytes as f64;
        let metric_name = MetricName::SourcesPayloadBytes.as_str();
        ::metrics::histogram!(metric_name).record(b);
        tokio::spawn(async move {
            // Push histogram with buckets instead of single value
            let _ = push_histogram_metric(metric_name, b).await;
        });
    }
    
    /// Record successful registry load
    pub fn registry_load_success() {
        counter_and_push!(MetricName::SourcesRegistryLoadsSuccess.as_str());
    }
    
    /// Record failed registry load
    pub fn registry_load_error() {
        counter_and_push!(MetricName::SourcesRegistryLoadsError.as_str());
    }
}

// ============================================================================
// Gateway Metrics
// ============================================================================

pub mod gateway {
    use super::{push_single_metric, MetricName};
    
    /// Record an accepted envelope
    pub fn envelope_accepted() {
        let metric_name = MetricName::GatewayEnvelopesAccepted.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record a deduplicated envelope
    pub fn envelope_deduplicated() {
        let metric_name = MetricName::GatewayEnvelopesDeduplicated.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record successful CAS write
    pub fn cas_write_success() {
        let metric_name = MetricName::GatewayCasWritesSuccess.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record failed CAS write
    pub fn cas_write_error() {
        let metric_name = MetricName::GatewayCasWritesError.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record ingested records
    #[allow(dead_code)]
    pub fn records_ingested(count: u64) {
        let metric_name = MetricName::GatewayRecordsIngested.as_str();
        ::metrics::counter!(metric_name).increment(count);
        let c = count as f64;
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, c, "counter").await;
        });
    }
    
    /// Record processing duration
    pub fn processing_duration(secs: f64) {
        let metric_name = MetricName::GatewayProcessingDuration.as_str();
        ::metrics::histogram!(metric_name).record(secs);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, secs, "gauge").await;
        });
    }
    
    /// Record successful ingest for a source
    pub fn ingest_success(source_id: &str) {
        ::metrics::counter!(MetricName::GatewayIngestSuccess.as_str(), "source_id" => source_id.to_string()).increment(1);
    }
    
    /// Record failed ingest for a source
    pub fn ingest_error(source_id: &str, error_type: &str) {
        ::metrics::counter!(MetricName::GatewayIngestError.as_str(), 
            "source_id" => source_id.to_string(),
            "error_type" => error_type.to_string()
        ).increment(1);
    }
    
    /// Record bytes ingested for a source
    pub fn bytes_ingested(source_id: &str, bytes: u64) {
        ::metrics::histogram!(MetricName::GatewayBytesIngested.as_str(), "source_id" => source_id.to_string()).record(bytes as f64);
    }
    
    /// Record ingest duration for a source
    pub fn duration(source_id: &str, secs: f64) {
        ::metrics::histogram!(MetricName::GatewayIngestDuration.as_str(), "source_id" => source_id.to_string()).record(secs);
    }
}

// ============================================================================
// Ingest Log Metrics
// ============================================================================

pub mod ingest_log {
    use super::{push_single_metric, MetricName};
    
    /// Record successful write
    pub fn write_success() {
        let metric_name = MetricName::IngestLogWritesSuccess.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record failed write
    pub fn write_error() {
        let metric_name = MetricName::IngestLogWritesError.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record write size
    pub fn write_bytes(bytes: usize) {
        let b = bytes as f64;
        let metric_name = MetricName::IngestLogWriteBytes.as_str();
        ::metrics::histogram!(metric_name).record(b);
        // Note: Don't push histogram to pushgateway - let Prometheus recorder handle bucket creation
    }
    
    /// Record log rotation
    #[allow(dead_code)]
    pub fn rotation() {
        ::metrics::counter!(MetricName::IngestLogRotations.as_str()).increment(1);
    }
    
    /// Set current file size
    pub fn current_file_bytes(bytes: u64) {
        let metric_name = MetricName::IngestLogCurrentFileBytes.as_str();
        ::metrics::gauge!(metric_name).set(bytes as f64);
        let b = bytes as f64;
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, b, "gauge").await;
        });
    }
    
    /// Set active consumers count
    #[allow(dead_code)]
    pub fn active_consumers(count: usize) {
        ::metrics::gauge!(MetricName::IngestLogActiveConsumers.as_str()).set(count as f64);
    }
}

// ============================================================================
// Parser Metrics
// ============================================================================

pub mod parser {
    use super::{push_single_metric, MetricName};
    
    /// Record successful parse
    pub fn parse_success() {
        let metric_name = MetricName::ParserParseSuccess.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record parse error
    pub fn parse_error() {
        let metric_name = MetricName::ParserParseError.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record parse duration
    pub fn duration(secs: f64) {
        ::metrics::histogram!(MetricName::ParserDuration.as_str()).record(secs);
    }
    
    /// Record extracted records
    pub fn records_extracted(count: u64) {
        let metric_name = MetricName::ParserRecordsExtracted.as_str();
        ::metrics::counter!(metric_name).increment(count);
        let c = count as f64;
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, c, "counter").await;
        });
    }
    
    /// Record bytes processed
    #[allow(dead_code)]
    pub fn bytes_processed(bytes: usize) {
        ::metrics::histogram!(MetricName::ParserBytesProcessed.as_str()).record(bytes as f64);
    }
    
    /// Record batch size
    pub fn batch_size(size: usize) {
        ::metrics::histogram!(MetricName::ParserBatchSize.as_str()).record(size as f64);
    }
}

// ============================================================================
// Normalize Metrics
// ============================================================================

pub mod normalize {
    use super::push_single_metric;
    
    /// Record that a record was normalized with a specific strategy
    pub fn record_normalized(strategy: &str) {
        let metric_name = "sms_normalize_records_processed_total";
        ::metrics::counter!(metric_name, "strategy" => strategy.to_string()).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record the confidence level of normalization
    pub fn confidence_recorded(confidence: f64) {
        ::metrics::histogram!("sms_normalize_confidence").record(confidence);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
    
    /// Record that geocoding was performed
    pub fn geocoding_performed() {
        let metric_name = "sms_normalize_geocoding_total";
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record a warning during normalization
    pub fn warning_logged(warning: &str) {
        let metric_name = "sms_normalize_warnings_total";
        ::metrics::counter!(metric_name, "warning_type" => warning.to_string()).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a batch was processed
    pub fn batch_processed(batch_size: usize) {
        ::metrics::histogram!("sms_normalize_batch_size").record(batch_size as f64);
        let metric_name = "sms_normalize_batches_processed_total";
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
}

// ============================================================================
// Quality Gate Metrics
// ============================================================================

pub mod quality_gate {
    use super::{push_single_metric, MetricName};
    
    /// Record that a record was accepted by the quality gate
    pub fn record_accepted() {
        let metric_name = MetricName::QualityGateRecordsAccepted.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a record was accepted with warnings by the quality gate
    pub fn record_accepted_with_warnings() {
        let metric_name = MetricName::QualityGateRecordsAcceptedWithWarnings.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a record was quarantined by the quality gate
    pub fn record_quarantined() {
        let metric_name = MetricName::QualityGateRecordsQuarantined.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record the quality score of an assessed record
    pub fn quality_score_recorded(score: f64) {
        let metric_name = MetricName::QualityGateQualityScore.as_str();
        ::metrics::histogram!(metric_name).record(score);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
    
    /// Record that a quality issue was detected
    pub fn issue_detected(issue_type: &str, severity: &str) {
        let metric_name = MetricName::QualityGateIssuesDetected.as_str();
        ::metrics::counter!(metric_name, 
            "issue_type" => issue_type.to_string(),
            "severity" => severity.to_string()
        ).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a batch was processed through the quality gate
    #[allow(dead_code)]
    pub fn batch_processed(total_records: usize, accepted_count: usize, quarantined_count: usize) {
        // Record batch size
        let metric_name = MetricName::QualityGateBatchSize.as_str();
        ::metrics::histogram!(metric_name).record(total_records as f64);
        
        // Record batch processing
        let batch_metric = MetricName::QualityGateBatchesProcessed.as_str();
        ::metrics::counter!(batch_metric, 
            "accepted" => accepted_count.to_string(),
            "quarantined" => quarantined_count.to_string()
        ).increment(1);
        
        tokio::spawn(async move {
            let _ = push_single_metric(batch_metric, 1.0, "counter").await;
        });
    }
}

// ============================================================================
// Pushgateway Support (for short-lived jobs)
// ============================================================================

/// Push ingest metrics - wrapper for compatibility with existing code
#[allow(dead_code)]
pub async fn push_ingest_metrics(
    source_id: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
    envelope_id: &str,
) {
    // Record the metrics locally first
    if success {
        gateway::ingest_success(source_id);
        gateway::bytes_ingested(source_id, bytes as u64);
        gateway::duration(source_id, duration_secs);
        ::metrics::counter!(MetricName::GatewayEnvelopeCreated.as_str(), 
            "source_id" => source_id.to_string(), 
            "envelope_id" => envelope_id.to_string()
        ).increment(1);
    } else {
        gateway::ingest_error(source_id, "fetch_failed");
    }
    
    // Try to push to pushgateway if configured
    if let Err(e) = push_to_pushgateway(source_id, bytes, duration_secs, success).await {
        warn!("Failed to push metrics to pushgateway: {}", e);
    }
}

/// Push metrics to Pushgateway
#[allow(dead_code)]
pub async fn push_to_pushgateway(
    instance: &str,
    bytes: usize,
    duration_secs: f64,
    success: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("SMS_PUSHGATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:9091".to_string());
    
    let push_url = format!(
        "{}/metrics/job/sms_scraper/instance/{}",
        base.trim_end_matches('/'),
        instance
    );
    
    // Create simple metrics in Prometheus text format
    let timestamp = chrono::Utc::now().timestamp_millis();
    let metrics_text = format!(
        "# HELP sms_ingest_timestamp_ms Last ingest timestamp\n\
         # TYPE sms_ingest_timestamp_ms gauge\n\
         sms_ingest_timestamp_ms {}\n\
         # HELP sms_ingest_bytes Total bytes ingested\n\
         # TYPE sms_ingest_bytes gauge\n\
         sms_ingest_bytes {}\n\
         # HELP sms_ingest_duration_seconds Ingest duration\n\
         # TYPE sms_ingest_duration_seconds gauge\n\
         sms_ingest_duration_seconds {}\n\
         # HELP sms_ingest_success Ingest success (1) or failure (0)\n\
         # TYPE sms_ingest_success gauge\n\
         sms_ingest_success {}\n",
        timestamp,
        bytes,
        duration_secs,
        if success { 1 } else { 0 }
    );
    
    let client = reqwest::Client::new();
    let response = client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Pushgateway returned status: {}", response.status()).into());
    }
    
    info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
    Ok(())
}

/// Push ALL collected metrics to Pushgateway
pub async fn push_all_metrics() -> Result<(), Box<dyn std::error::Error>> {
    push_all_metrics_with_instance("default").await
}

/// Push ALL collected metrics to Pushgateway with custom instance label
pub async fn push_all_metrics_with_instance(instance: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pushgateway_url = std::env::var("SMS_PUSHGATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:9091".to_string());
    
    let push_url = format!(
        "{}/metrics/job/sms_scraper/instance/{}",
        pushgateway_url.trim_end_matches('/'),
        instance
    );
    
    // Build metrics text by iterating through known metrics
    // This is a simplified approach that won't capture histogram buckets,
    // but will ensure all basic metrics are pushed
    let mut metrics_text = String::new();
    
    // Add a timestamp marker
    let timestamp = chrono::Utc::now().timestamp_millis();
    metrics_text.push_str(&format!(
        "# HELP sms_push_timestamp_ms Last push timestamp\n\
         # TYPE sms_push_timestamp_ms gauge\n\
         sms_push_timestamp_ms {}\n",
        timestamp
    ));
    
    // Try to render from the handle if available
    if let Some(state) = METRICS_HANDLE.get() {
        // Try to get rendered metrics directly
        let rendered = state.handle.render();
        if !rendered.is_empty() {
            info!("Rendered {} bytes of metrics directly", rendered.len());
            metrics_text.push_str(&rendered);
        } else {
            // If render() returns empty, we'll push a marker indicating metrics are initialized
            metrics_text.push_str(
                "# HELP sms_metrics_initialized Whether metrics system is initialized\n\
                 # TYPE sms_metrics_initialized gauge\n\
                 sms_metrics_initialized 1\n"
            );
        }
    } else {
        warn!("Metrics not initialized with push gateway support.");
        metrics_text.push_str(
            "# HELP sms_metrics_initialized Whether metrics system is initialized\n\
             # TYPE sms_metrics_initialized gauge\n\
             sms_metrics_initialized 0\n"
        );
    }
    
    let client = reqwest::Client::new();
    let response = client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Pushgateway returned status {}: {}", status, body).into());
    }
    
    info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
    Ok(())
}

// ============================================================================
// Enrich Metrics
// ============================================================================

pub mod enrich {
    use super::{push_single_metric, MetricName};
    
    /// Record that a record was enriched with a specific strategy
    pub fn record_enriched(strategy: &str) {
        let metric_name = MetricName::EnrichRecordsProcessed.as_str();
        ::metrics::counter!(metric_name, "strategy" => strategy.to_string()).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record the confidence level of enrichment
    pub fn confidence_recorded(confidence: f64) {
        let metric_name = MetricName::EnrichConfidence.as_str();
        ::metrics::histogram!(metric_name).record(confidence);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
    
    /// Record that spatial binning was performed
    pub fn spatial_binning_performed() {
        let metric_name = MetricName::EnrichSpatialBinning.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that city tagging was performed
    pub fn city_tagging_performed() {
        let metric_name = MetricName::EnrichCityTagging.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record the number of tags added to a record
    pub fn tags_added(count: usize) {
        let metric_name = MetricName::EnrichTagsAdded.as_str();
        ::metrics::histogram!(metric_name).record(count as f64);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
    
    /// Record a warning during enrichment
    pub fn warning_logged(warning: &str) {
        let metric_name = MetricName::EnrichWarnings.as_str();
        ::metrics::counter!(metric_name, "warning_type" => warning.to_string()).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a batch was processed through enrichment
    #[allow(dead_code)]
    pub fn batch_processed(batch_size: usize) {
        // Record batch size
        let metric_name = MetricName::EnrichBatchSize.as_str();
        ::metrics::histogram!(metric_name).record(batch_size as f64);
        
        // Record batch processing
        let batch_metric = MetricName::EnrichBatchesProcessed.as_str();
        ::metrics::counter!(batch_metric).increment(1);
        
        tokio::spawn(async move {
            let _ = push_single_metric(batch_metric, 1.0, "counter").await;
        });
    }
}

// ============================================================================
// Conflation Metrics
// ============================================================================

pub mod conflation {
    use super::{push_single_metric, MetricName};
    
    /// Record that a record was processed through conflation
    pub fn records_processed() {
        let metric_name = MetricName::ConflationRecordsProcessed.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a record was successfully conflated
    pub fn records_successful() {
        let metric_name = MetricName::ConflationRecordsSuccessful.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that conflation failed for a record
    pub fn records_failed() {
        let metric_name = MetricName::ConflationRecordsFailed.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record the confidence score of conflation
    pub fn confidence_score_recorded(confidence: f64) {
        let metric_name = MetricName::ConflationConfidenceScore.as_str();
        ::metrics::histogram!(metric_name).record(confidence);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
    
    /// Record that a new entity was created
    pub fn new_entity_created() {
        let metric_name = MetricName::ConflationNewEntities.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a record matched an existing entity
    pub fn matched_existing() {
        let metric_name = MetricName::ConflationMatchedExisting.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that an existing entity was updated
    pub fn updated_existing() {
        let metric_name = MetricName::ConflationUpdatedExisting.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that a duplicate was detected
    pub fn duplicate_detected() {
        let metric_name = MetricName::ConflationDuplicates.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record that conflation resulted in uncertain resolution
    pub fn uncertain_resolution() {
        let metric_name = MetricName::ConflationUncertainResolutions.as_str();
        ::metrics::counter!(metric_name).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record a warning during conflation
    pub fn warning_logged(warning: &str) {
        let metric_name = MetricName::ConflationWarnings.as_str();
        ::metrics::counter!(metric_name, "warning_type" => warning.to_string()).increment(1);
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, 1.0, "counter").await;
        });
    }
    
    /// Record potential duplicates found
    pub fn potential_duplicates(count: usize) {
        let metric_name = MetricName::ConflationPotentialDuplicates.as_str();
        ::metrics::counter!(metric_name).increment(count as u64);
        let c = count as f64;
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, c, "counter").await;
        });
    }
    
    /// Record alternative matches found
    pub fn alternative_matches(count: usize) {
        let metric_name = MetricName::ConflationAlternativeMatches.as_str();
        ::metrics::counter!(metric_name).increment(count as u64);
        let c = count as f64;
        tokio::spawn(async move {
            let _ = push_single_metric(metric_name, c, "counter").await;
        });
    }
    
    /// Record batch processing metrics
    pub fn batch_processed(batch_size: usize, successful_count: usize, failed_count: usize) {
        // Record batch size
        let batch_size_metric = MetricName::ConflationBatchSize.as_str();
        ::metrics::histogram!(batch_size_metric).record(batch_size as f64);
        
        // Record batch processing
        let batches_processed = MetricName::ConflationBatchesProcessed.as_str();
        ::metrics::counter!(batches_processed).increment(1);
        
        // Record batch success
        if failed_count == 0 {
            let batches_successful = MetricName::ConflationBatchesSuccessful.as_str();
            ::metrics::counter!(batches_successful).increment(1);
            tokio::spawn(async move {
                let _ = push_single_metric(batches_successful, 1.0, "counter").await;
            });
        }
        
        // Record individual record results
        let records_successful = MetricName::ConflationBatchRecordsSuccessful.as_str();
        ::metrics::counter!(records_successful).increment(successful_count as u64);
        
        let records_failed = MetricName::ConflationBatchRecordsFailed.as_str();
        ::metrics::counter!(records_failed).increment(failed_count as u64);
        
        tokio::spawn(async move {
            let _ = push_single_metric(batches_processed, 1.0, "counter").await;
            let _ = push_single_metric(records_successful, successful_count as f64, "counter").await;
            let _ = push_single_metric(records_failed, failed_count as f64, "counter").await;
        });
    }
    
    /// Record batch processing duration
    pub fn batch_processing_duration(duration_seconds: f64) {
        let metric_name = MetricName::ConflationBatchProcessingDuration.as_str();
        ::metrics::histogram!(metric_name).record(duration_seconds);
        // Don't push histograms to pushgateway - let Prometheus handle aggregation
    }
}
