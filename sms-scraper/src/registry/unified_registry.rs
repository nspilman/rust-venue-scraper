use anyhow::Result;

use super::source_loader::{SourceConfig, SourceRegistry};
use crate::app::ports::{ParserFactory, ParserPort};
use crate::infra::parser_factory::DefaultParserFactory;
use crate::pipeline::processing::normalize::registry::NormalizationRegistry;
use crate::pipeline::processing::normalize::normalizers::SourceNormalizer;

/// Unified registry that connects sources to their parsers and normalizers
pub struct UnifiedSourceRegistry {
    source_registry: SourceRegistry,
    parser_factory: Box<dyn ParserFactory>,
    normalizer_registry: NormalizationRegistry,
}

impl UnifiedSourceRegistry {
    /// Create a new unified registry from a source registry directory
    pub fn new(registry_dir: &str) -> Result<Self> {
        let source_registry = SourceRegistry::load_from_directory(registry_dir)
            .map_err(|e| anyhow::anyhow!("Failed to load source registry: {}", e))?;
        
        let parser_factory = Box::new(DefaultParserFactory);
        let normalizer_registry = NormalizationRegistry::new();
        
        Ok(Self {
            source_registry,
            parser_factory,
            normalizer_registry,
        })
    }
    
    /// Get source configuration for a given source ID
    pub fn get_source_config(&self, source_id: &str) -> Result<&SourceConfig> {
        self.source_registry.get_source_config(source_id)
            .ok_or_else(|| anyhow::anyhow!("Source not found: {}", source_id))
    }
    
    /// Get parser for a source using either new pipeline config or fallback to parse_plan_ref
    pub fn get_parser_for_source(&self, source_id: &str) -> Result<Box<dyn ParserPort>> {
        let source_config = self.get_source_config(source_id)?;
        
        // Try new pipeline config first
        if let Some(pipeline) = &source_config.pipeline {
            let parse_plan = format!("parse_plan:{}", pipeline.parser_id);
            return self.parser_factory.for_plan(&parse_plan)
                .ok_or_else(|| anyhow::anyhow!("Parser not found for plan: {}", parse_plan));
        }
        
        // Fallback to existing parse_plan_ref for backward compatibility
        if let Some(parse_plan_ref) = &source_config.parse_plan_ref {
            return self.parser_factory.for_plan(parse_plan_ref)
                .ok_or_else(|| anyhow::anyhow!("Parser not found for plan: {}", parse_plan_ref));
        }
        
        Err(anyhow::anyhow!("No parser configuration found for source: {}", source_id))
    }
    
    /// Get normalizer for a source using either new pipeline config or fallback to source_id
    pub fn get_normalizer_for_source(&self, source_id: &str) -> Result<&dyn SourceNormalizer> {
        let source_config = self.get_source_config(source_id)?;
        
        // Try new pipeline config first
        if let Some(pipeline) = &source_config.pipeline {
            return self.normalizer_registry.get_normalizer(&pipeline.normalizer_id)
                .ok_or_else(|| anyhow::anyhow!("Normalizer not found: {}", pipeline.normalizer_id));
        }
        
        // Fallback to using source_id as normalizer_id (current behavior)
        self.normalizer_registry.get_normalizer(source_id)
            .ok_or_else(|| anyhow::anyhow!("Normalizer not found for source: {}", source_id))
    }
    
    /// Check if a source is enabled
    pub fn is_source_enabled(&self, source_id: &str) -> bool {
        self.source_registry.is_source_enabled(source_id)
    }
    
    /// Get all enabled source IDs
    pub fn get_enabled_sources(&self) -> Vec<String> {
        self.source_registry.get_enabled_sources()
    }
    
    /// Get primary URL for a source (delegated to source registry)
    pub fn get_source_url(&self, source_id: &str) -> Result<String> {
        self.source_registry.get_source_url(source_id)
            .map_err(|e| anyhow::anyhow!("Failed to get source URL: {}", e))
    }
}

