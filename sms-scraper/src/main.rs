use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::info;
use dotenv;

use sms_core::storage::database::DatabaseStorage;
use sms_core::storage::traits::Storage;

use sms_scraper::pipeline::{FullPipelineOrchestrator, PipelineOrchestrator, PipelineConfig};

#[derive(Parser)]
#[command(name = "sms-scraper")]
#[command(about = "SMS scraper with all crawlers and processing pipeline")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run data ingestion for specified APIs
    Ingester {
        /// Comma-separated list of APIs to run
        #[arg(long)]
        apis: String,
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
    },
    /// Run a full pipeline for a source
    #[command(name = "full-pipeline")]
    FullPipeline {
        #[arg(long)]
        source_id: String,
        #[arg(long, default_value = "false")]
        bypass_cadence: bool,
    },
    /// Run a modular pipeline for a source (new architecture)
    #[command(name = "modular-pipeline")]
    ModularPipeline {
        #[arg(long)]
        source_id: String,
        #[arg(long, default_value = "false")]
        parse_only: bool,
        #[arg(long, default_value = "false")]
        ingestion_only: bool,
    },
    /// Reprocess all existing raw data for a source (ignores processed flag)
    ReprocessAll {
        /// Source ID to reprocess
        #[arg(long)]
        source_id: String,
    },
    /// Clear data from the database
    ClearDb {
        /// Optional: Delete only data for a specific venue by slug (e.g., "neumos")
        /// If not provided, will clear ALL data from the database
        #[arg(long)]
        venue_slug: Option<String>,
    },
    /// Step 4: Parse envelopes from ingest log into neutral records
    Parse {
        /// Explicit input file path(s), comma-separated
        #[arg(long)]
        input: Option<String>,
        /// Process files for specific source
        #[arg(long)]
        source: Option<String>,
        /// Process files for all sources
        #[arg(long)]
        all_sources: bool,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Step 5: Normalize parsed records into standardized entities
    Normalize {
        /// Explicit input file path(s), comma-separated
        #[arg(long)]
        input: Option<String>,
        /// Process files for specific source
        #[arg(long)]
        source: Option<String>,
        /// Process files for all sources
        #[arg(long)]
        all_sources: bool,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Step 6: Apply quality gates to normalized data
    QualityGate {
        /// Explicit input file path(s), comma-separated
        #[arg(long)]
        input: Option<String>,
        /// Process files for specific sources, comma-separated
        #[arg(long)]
        sources: Option<String>,
        /// Process files for all sources
        #[arg(long)]
        all_sources: bool,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Step 7: Enrich data with additional information
    Enrich {
        /// Explicit input file path(s), comma-separated
        #[arg(long)]
        input: Option<String>,
        /// Process files for specific source
        #[arg(long)]
        source: Option<String>,
        /// Process files for all sources
        #[arg(long)]
        all_sources: bool,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Step 8: Conflate entities across sources to resolve duplicates
    Conflation {
        /// Explicit input file path(s), comma-separated
        #[arg(long)]
        input: Option<String>,
        /// Process files for specific sources, comma-separated
        #[arg(long)]
        sources: Option<String>,
        /// Process all enriched files
        #[arg(long)]
        all_enriched: bool,
        /// Confidence threshold for matching (0.0-1.0)
        #[arg(long, default_value = "0.8")]
        confidence_threshold: f64,
        /// Output file path
        #[arg(long)]
        output: Option<String>,
    },
    /// Step 9: Catalog final entities into the graph database
    Catalog {
        /// Explicit input file path
        #[arg(long)]
        input: Option<String>,
        /// Use latest conflated file
        #[arg(long)]
        latest: bool,
        /// Validate graph integrity after cataloging
        #[arg(long)]
        validate_graph: bool,
        /// Storage mode: "memory" or "database"
        #[arg(long, default_value = "database")]
        storage_mode: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Initialize database storage
    info!("Initializing database storage...");
    let storage: Arc<dyn Storage> = Arc::new(DatabaseStorage::new().await?);
    info!("Database storage initialized successfully");

    match cli.command {
        Commands::Ingester { apis, bypass_cadence } => {
            println!("🕷️  Starting SMS scraper ingestion for APIs: {}", apis);
            
            // Set bypass cadence environment variable if requested
            if bypass_cadence {
                std::env::set_var("SMS_BYPASS_CADENCE", "1");
                println!("🚀 Bypassing cadence restrictions");
            }
            
            // Create orchestrator and run ingestion
            let orchestrator = FullPipelineOrchestrator::new().await?;
            
            // Parse the comma-separated API list
            let api_list: Vec<&str> = apis.split(',').map(|s| s.trim()).collect();
            
            for api_name in api_list {
                println!("🔄 Running ingestion for: {}", api_name);
                if let Err(e) = orchestrator.run_ingestion_for_source(api_name).await {
                    eprintln!("❌ Ingestion failed for {}: {}", api_name, e);
                } else {
                    println!("✅ Ingestion completed for: {}", api_name);
                }
            }
        }
        Commands::FullPipeline { source_id, bypass_cadence } => {
            println!("🔄 Running full pipeline for source: {}", source_id);
            
            // Set bypass cadence environment variable if requested
            if bypass_cadence {
                std::env::set_var("SMS_BYPASS_CADENCE", "1");
                println!("🚀 Bypassing cadence restrictions");
            }
            
            // Create the full pipeline orchestrator
            match FullPipelineOrchestrator::new().await {
                Ok(orchestrator) => {
                    // Process the source through the complete pipeline
                    match orchestrator.process_source(&source_id).await {
                        Ok(result) => {
                            println!("📊 Pipeline Results for {}:", result.source_id);
                            println!("   📁 Total items: {}", result.total_items);
                            println!("   ✅ Processed: {}", result.processed_items);
                            println!("   ❌ Failed: {}", result.failed_items);
                            println!("   📈 Success rate: {:.1}%", result.success_rate());
                            
                            if !result.errors.is_empty() {
                                println!("   🚨 Errors encountered:");
                                for error in &result.errors {
                                    println!("      - {}", error);
                                }
                            }
                            
                            if result.is_success() {
                                println!("✅ Pipeline completed successfully - entities created/updated in database");
                            } else {
                                println!("⚠️  Pipeline completed with errors - check logs for details");
                            }
                        }
                        Err(e) => {
                            tracing::error!("Pipeline processing failed for {}: {}", source_id, e);
                            println!("❌ Pipeline failed for {}: {}", source_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create pipeline orchestrator: {}", e);
                    println!("❌ Failed to initialize pipeline: {}", e);
                }
            }
        }
        Commands::ReprocessAll { source_id } => {
            println!("🔄 Reprocessing ALL raw data for source: {}", source_id);
            
            println!("❌ ReprocessAll command temporarily disabled due to compilation issues");
            println!("💡 Use the clear-db and full-pipeline commands instead:");
            println!("   1. cargo run --bin sms-scraper -- clear-db --venue-slug {}", source_id);
            println!("   2. cargo run --bin sms-scraper -- full-pipeline --source-id {}", source_id);
        }
        Commands::ClearDb { venue_slug } => {
            use sms_core::database::DatabaseManager;
            
            println!("🗑️  Clearing database data...");
            
            if let Some(slug) = venue_slug {
                println!("🗑️  Deleting data for venue '{}'...", slug);
                println!("⚠️  WARNING: This will permanently delete the venue and all its events!");
                
                // Create a direct database manager instance to access delete methods
                match DatabaseManager::new().await {
                    Ok(db_manager) => {
                        // Use the simplified delete method from sms-core
                        match db_manager.delete_venue_data(&slug).await {
                            Ok(()) => {
                                println!("✅ Successfully deleted all data for venue '{}'!", slug);
                            }
                            Err(e) => {
                                tracing::error!("Failed to delete venue data: {}", e);
                                println!("❌ Failed to delete venue data: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create database manager: {}", e);
                        println!("❌ Failed to connect to database: {}", e);
                    }
                }
            } else {
                println!("🗑️  Clearing ALL data from the database...");
                println!("⚠️  WARNING: This will permanently delete ALL data!");
                println!("💡 Tip: Use --venue-slug to delete data for a specific venue only");
                println!("❌ Full database clear not implemented in this command");
                println!("   Use the database management tools directly if needed");
            }
        }
        Commands::Parse { input, source, all_sources, output } => {
            println!("📄 Step 4: Parse - Converting raw data to neutral records");
            
            if let Some(source_id) = source {
                match FullPipelineOrchestrator::new().await {
                    Ok(orchestrator) => {
                        match orchestrator.run_parse_for_source(&source_id).await {
                            Ok(()) => {
                                println!("✅ Parse completed successfully for {}", source_id);
                            }
                            Err(e) => {
                                tracing::error!("Parse failed for {}: {}", source_id, e);
                                println!("❌ Parse failed for {}: {}", source_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create pipeline orchestrator: {}", e);
                        println!("❌ Failed to initialize pipeline: {}", e);
                    }
                }
            } else {
                println!("❌ Please specify a source with --source <source_name>");
                println!("📋 Available sources: blue_moon, barboza, neumos, etc.");
            }
        }
        Commands::Normalize { input, source, all_sources, output } => {
            println!("🔧 Step 5: Normalize - Standardizing parsed data");
            
            if let Some(source_id) = source {
                match FullPipelineOrchestrator::new().await {
                    Ok(orchestrator) => {
                        match orchestrator.run_normalize_for_source(&source_id).await {
                            Ok(()) => {
                                println!("✅ Normalize completed successfully for {}", source_id);
                            }
                            Err(e) => {
                                tracing::error!("Normalize failed for {}: {}", source_id, e);
                                println!("❌ Normalize failed for {}: {}", source_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create pipeline orchestrator: {}", e);
                        println!("❌ Failed to initialize pipeline: {}", e);
                    }
                }
            } else {
                println!("❌ Please specify a source with --source <source_name>");
                println!("📋 Available sources: blue_moon, barboza, neumos, etc.");
            }
        }
        Commands::QualityGate { input: _, sources, all_sources: _, output: _ } => {
            println!("🛡️ Step 6: Quality Gate - Validating data quality");
            
            if let Some(source_id) = sources {
                match FullPipelineOrchestrator::new().await {
                    Ok(orchestrator) => {
                        match orchestrator.run_quality_gate_for_source(&source_id).await {
                            Ok(()) => {
                                println!("✅ Quality gate completed successfully for {}", source_id);
                            }
                            Err(e) => {
                                tracing::error!("Quality gate failed for {}: {}", source_id, e);
                                println!("❌ Quality gate failed for {}: {}", source_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create pipeline orchestrator: {}", e);
                        println!("❌ Failed to initialize pipeline: {}", e);
                    }
                }
            } else {
                println!("❌ Please specify a source with --sources <source_name>");
                println!("📋 Available sources: blue_moon, barboza, neumos, etc.");
            }
        }
        Commands::Enrich { input: _, source, all_sources: _, output: _ } => {
            println!("🌐 Step 7: Enrich - Adding location and metadata");
            
            if let Some(source_id) = source {
                match FullPipelineOrchestrator::new().await {
                    Ok(orchestrator) => {
                        match orchestrator.run_enrich_for_source(&source_id).await {
                            Ok(()) => {
                                println!("✅ Enrich completed successfully for {}", source_id);
                            }
                            Err(e) => {
                                tracing::error!("Enrich failed for {}: {}", source_id, e);
                                println!("❌ Enrich failed for {}: {}", source_id, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to create pipeline orchestrator: {}", e);
                        println!("❌ Failed to initialize pipeline: {}", e);
                    }
                }
            } else {
                println!("❌ Please specify a source with --source <source_name>");
                println!("📋 Available sources: blue_moon, barboza, neumos, etc.");
            }
        }
        Commands::Conflation { input: _, sources, all_enriched: _, confidence_threshold, output: _ } => {
            println!("🔗 Step 8: Conflation - Resolving duplicate entities");
            println!("🔧 Confidence threshold: {}", confidence_threshold);
            
            if let Some(source_list) = sources {
                for source_id in source_list.split(',').map(|s| s.trim()) {
                    match FullPipelineOrchestrator::new().await {
                        Ok(orchestrator) => {
                            match orchestrator.run_conflation_for_source(source_id, confidence_threshold).await {
                                Ok(()) => {
                                    println!("✅ Conflation completed successfully for {}", source_id);
                                }
                                Err(e) => {
                                    tracing::error!("Conflation failed for {}: {}", source_id, e);
                                    println!("❌ Conflation failed for {}: {}", source_id, e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to create pipeline orchestrator: {}", e);
                            println!("❌ Failed to initialize pipeline: {}", e);
                        }
                    }
                }
            } else {
                println!("❌ Please specify sources with --sources <source_names>");
                println!("📋 Available sources: blue_moon, barboza, neumos, etc.");
                println!("💡 Example: --sources blue_moon,barboza");
            }
        }
        Commands::Catalog { input: _, latest: _, validate_graph, storage_mode: _ } => {
            println!("📚 Step 9: Catalog - Storing entities in graph database");
            println!("✅ Validate graph: {}", validate_graph);
            
            // For now, catalog blue_moon as example - in future could support --sources parameter
            let source_id = "blue_moon";
            match FullPipelineOrchestrator::new().await {
                Ok(orchestrator) => {
                    match orchestrator.run_catalog_for_source(source_id, validate_graph).await {
                        Ok(()) => {
                            println!("✅ Catalog completed successfully for {}", source_id);
                        }
                        Err(e) => {
                            tracing::error!("Catalog failed for {}: {}", source_id, e);
                            println!("❌ Catalog failed for {}: {}", source_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create pipeline orchestrator: {}", e);
                    println!("❌ Failed to initialize pipeline: {}", e);
                }
            }
        }
        Commands::ModularPipeline { source_id, parse_only, ingestion_only } => {
            println!("🚀 Running modular pipeline for source: {}", source_id);
            
            match PipelineOrchestrator::new().await {
                Ok(orchestrator) => {
                    let config = if ingestion_only {
                        println!("🔄 Running ingestion only");
                        PipelineConfig::parse_only() // Will create ingestion-only config later
                    } else if parse_only {
                        println!("📄 Running parse only");
                        PipelineConfig::parse_only()
                    } else {
                        println!("🔄 Running full modular pipeline");
                        PipelineConfig::default_full_pipeline()
                    };
                    
                    match orchestrator.run_pipeline(config, &source_id).await {
                        Ok(result) => {
                            println!("✅ Modular pipeline completed successfully!");
                            println!("📊 Total processed: {}, failed: {}", result.total_processed, result.total_failed);
                            if let Some(duration) = result.duration() {
                                println!("⏱️ Duration: {}ms", duration.num_milliseconds());
                            }
                        }
                        Err(e) => {
                            tracing::error!("Modular pipeline failed for {}: {}", source_id, e);
                            println!("❌ Modular pipeline failed for {}: {}", source_id, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create modular pipeline orchestrator: {}", e);
                    println!("❌ Failed to initialize modular pipeline: {}", e);
                }
            }
        }
    }
    
    Ok(())
}
