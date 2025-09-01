use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a complete pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub description: String,
    pub steps: Vec<PipelineStepConfig>,
    pub error_handling: ErrorHandlingStrategy,
    pub parallel_execution: bool,
    pub metadata: HashMap<String, String>,
}

/// Configuration for individual pipeline steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStepConfig {
    Ingestion,
    Parse,
    Normalize,
    QualityGate { 
        threshold: Option<f64> 
    },
    Enrich,
    Conflation { 
        confidence_threshold: Option<f64> 
    },
    Catalog { 
        validate_graph: bool 
    },
}

/// Strategy for handling errors during pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// Stop pipeline execution on first error
    StopOnFirstError,
    /// Continue pipeline execution, collect errors
    ContinueOnError,
    /// Skip failed items but continue processing
    SkipFailedItems,
}

impl PipelineConfig {
    /// Create the default full pipeline configuration
    pub fn default_full_pipeline() -> Self {
        Self {
            name: "full_pipeline".to_string(),
            description: "Complete event scraping pipeline from ingestion to catalog".to_string(),
            steps: vec![
                PipelineStepConfig::Ingestion,
                PipelineStepConfig::Parse,
                PipelineStepConfig::Normalize,
                PipelineStepConfig::QualityGate { threshold: Some(0.8) },
                PipelineStepConfig::Enrich,
                PipelineStepConfig::Conflation { confidence_threshold: Some(0.85) },
                PipelineStepConfig::Catalog { validate_graph: true },
            ],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a parse-only pipeline configuration
    pub fn parse_only() -> Self {
        Self {
            name: "parse_only".to_string(),
            description: "Parse raw data into structured events".to_string(),
            steps: vec![PipelineStepConfig::Parse],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a normalize-only pipeline configuration
    pub fn normalize_only() -> Self {
        Self {
            name: "normalize_only".to_string(),
            description: "Normalize parsed events into consistent format".to_string(),
            steps: vec![PipelineStepConfig::Normalize],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a quality gate pipeline configuration
    pub fn quality_gate_only(threshold: Option<f64>) -> Self {
        Self {
            name: "quality_gate_only".to_string(),
            description: "Run quality gate validation on normalized events".to_string(),
            steps: vec![PipelineStepConfig::QualityGate { threshold }],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create an enrichment-only pipeline configuration
    pub fn enrich_only() -> Self {
        Self {
            name: "enrich_only".to_string(),
            description: "Enrich events with additional metadata".to_string(),
            steps: vec![PipelineStepConfig::Enrich],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a conflation-only pipeline configuration
    pub fn conflation_only(confidence_threshold: Option<f64>) -> Self {
        Self {
            name: "conflation_only".to_string(),
            description: "Resolve duplicate entities and create canonical IDs".to_string(),
            steps: vec![PipelineStepConfig::Conflation { confidence_threshold }],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a catalog-only pipeline configuration
    pub fn catalog_only(validate_graph: bool) -> Self {
        Self {
            name: "catalog_only".to_string(),
            description: "Store entities in graph database".to_string(),
            steps: vec![PipelineStepConfig::Catalog { validate_graph }],
            error_handling: ErrorHandlingStrategy::ContinueOnError,
            parallel_execution: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Validate the pipeline configuration
    pub fn validate(&self) -> Result<()> {
        if self.steps.is_empty() {
            return Err(anyhow::anyhow!("Pipeline must have at least one step"));
        }
        
        // Check for dependency violations
        let mut seen_steps = std::collections::HashSet::new();
        
        for step in &self.steps {
            let step_name = step.step_name();
            let dependencies = step.dependencies();
            
            // Check if all dependencies have been seen
            for dep in dependencies {
                if !seen_steps.contains(dep) {
                    return Err(anyhow::anyhow!(
                        "Step '{}' depends on '{}' which appears later in the pipeline",
                        step_name, dep
                    ));
                }
            }
            
            seen_steps.insert(step_name);
        }
        
        Ok(())
    }
}

impl PipelineStepConfig {
    /// Get the step name for dependency checking
    pub fn step_name(&self) -> &'static str {
        match self {
            PipelineStepConfig::Ingestion => "ingestion",
            PipelineStepConfig::Parse => "parse",
            PipelineStepConfig::Normalize => "normalize",
            PipelineStepConfig::QualityGate { .. } => "quality_gate",
            PipelineStepConfig::Enrich => "enrich",
            PipelineStepConfig::Conflation { .. } => "conflation",
            PipelineStepConfig::Catalog { .. } => "catalog",
        }
    }
    
    /// Get the dependencies for this step
    pub fn dependencies(&self) -> Vec<&'static str> {
        match self {
            PipelineStepConfig::Ingestion => vec![],
            PipelineStepConfig::Parse => vec!["ingestion"],
            PipelineStepConfig::Normalize => vec!["parse"],
            PipelineStepConfig::QualityGate { .. } => vec!["normalize"],
            PipelineStepConfig::Enrich => vec!["quality_gate"],
            PipelineStepConfig::Conflation { .. } => vec!["enrich"],
            PipelineStepConfig::Catalog { .. } => vec!["conflation"],
        }
    }
}
