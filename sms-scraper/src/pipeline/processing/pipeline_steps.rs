use std::path::PathBuf;
use anyhow::{Context, Result};
use sms_core::domain::RawData;
use tracing::{debug, info};

// App modules
use crate::app::{
    normalize_use_case::NormalizeUseCase,
    quality_gate_use_case::QualityGateUseCase,
};

// Infrastructure adapters
use crate::infra::{
    normalize_output_adapter::FileNormalizeOutputAdapter,
    quality_gate_output_adapter::{FileQualityGateOutputAdapter, QualityPartition},
};

// Test-only imports
#[cfg(test)]
use crate::app::quality_gate_use_case::QualityGateUseCase;

// Re-export types for convenience
pub use crate::pipeline::processing::{
    normalize::NormalizedRecord,
    parser::ParsedRecord,
    quality_gate::{QualityAssessedRecord, QualityDecision},
};

/// Process a raw data item through the normalization and quality gate steps
pub async fn process_raw_data(
    raw_data: &RawData,
    output_dir: &str,
) -> Result<ProcessedData> {
    info!("Processing raw data: {} - {}", raw_data.event_name, raw_data.id.map(|id| id.to_string()).unwrap_or_else(|| "<no-id>".to_string()));
    
    // Create output directory if it doesn't exist
    let output_path = PathBuf::from(output_dir);
    if !output_path.exists() {
        tokio::fs::create_dir_all(&output_path).await
            .context("Failed to create output directory")?;
    }
    
    // Initialize output adapters
    let normalize_output = match FileNormalizeOutputAdapter::new(output_dir) {
        Ok(adapter) => adapter,
        Err(e) => anyhow::bail!("Failed to create normalize output adapter: {}", e),
    };
    
    let accepted_path = output_path.join("accepted");
    let quarantined_path = output_path.join("quarantined");
    
    // Ensure output directories exist
    tokio::fs::create_dir_all(&accepted_path).await
        .context("Failed to create accepted directory")?;
    tokio::fs::create_dir_all(&quarantined_path).await
        .context("Failed to create quarantined directory")?;
    
    let _accepted_output = Box::new(FileQualityGateOutputAdapter::new(accepted_path, QualityPartition::Accepted));
    let _quarantined_output = Box::new(FileQualityGateOutputAdapter::new(quarantined_path, QualityPartition::Quarantined));

    // Initialize use cases
    let normalize_use_case = NormalizeUseCase::new(Box::new(normalize_output));
    
    // Always use the actual quality gate implementation
    let quality_gate = {
        use crate::app::quality_gate_use_case::QualityGateUseCase;
        Some(QualityGateUseCase::with_default_quality_gate(
            _accepted_output,
            _quarantined_output,
        ))
    };

    // Step 1: Parse the raw data
    let parsed = parse_raw_data(raw_data).await
        .context("Failed to parse raw data")?;

    // Step 2: Normalize the parsed data
    let normalized = normalize_use_case.normalize_record(&parsed).await
        .context("Failed to normalize data")?;
    
    // Get the first normalized record if any
    let first_normalized = normalized.into_iter().next();
    
    // Step 3: Apply quality gate if available and we have a normalized record
    let quality_assessed = match (quality_gate, first_normalized.clone()) {
        (Some(quality_gate), Some(normalized_record)) => {
            let assessed = quality_gate.assess_record(&normalized_record).await
                .context("Failed to assess data quality")?;
            Some(assessed)
        }
        _ => None,
    };
    
    // Create the result with all processing steps
    let result = ProcessedData {
        raw_data: raw_data.clone(),
        parsed,
        normalized: first_normalized,
        quality_assessed,
    };
    
    // Log the final result
    info!(
        "Processed data - Parsed: {}, Normalized: {}, Quality: {:?}",
        result.parsed.record_path,
        result.normalized.is_some(),
        result.quality_assessed.as_ref().map(|q| &q.quality_assessment.decision)
    );
    
    Ok(result)
}

/// Parse raw data into a structured format
async fn parse_raw_data(raw_data: &RawData) -> Result<ParsedRecord> {
    debug!("Parsing raw data for event: {}", raw_data.event_name);
    
    // Create a parsed record with the raw data
    // In a real implementation, we would parse the raw data into a more structured format
    // using the appropriate parser for the source
    // Convert internal API name to external source ID for normalization
    let source_id = match raw_data.api_name.as_str() {
        "crawler_blue_moon" => "blue_moon",
        "crawler_sea_monster" => "sea_monster", 
        "crawler_darrells_tavern" => "darrells_tavern",
        "crawler_barboza" => "barboza",
        "crawler_kexp" => "kexp",
        "crawler_neumos" => "neumos",
        "crawler_conor_byrne" => "conor_byrne",
        other => other, // Fallback for any other source
    };

    let parsed = ParsedRecord {
        source_id: source_id.to_string(),
        envelope_id: raw_data.id.map(|id| id.to_string()).unwrap_or_default(),
        payload_ref: raw_data.api_name.clone(),
        record_path: format!("/events/{}", raw_data.event_api_id),
        record: raw_data.data.clone(),
    };
    
    // Log parsing result
    debug!("Parsed record: {:?}", parsed);
    
    Ok(parsed)
}

/// Represents the result of processing a raw data item through all pipeline steps
#[derive(Debug)]
pub struct ProcessedData {
    pub raw_data: RawData,
    pub parsed: ParsedRecord,
    pub normalized: Option<NormalizedRecord>,
    pub quality_assessed: Option<QualityAssessedRecord>,
}

// Using the ParsedRecord from the parser module instead of defining it here
