use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::{info, error};
use dotenv;

use sms_core::storage::database::DatabaseStorage;
use sms_core::storage::traits::Storage;

use sms_scraper::pipeline::FullPipelineOrchestrator;

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
    /// Run the complete pipeline for a source
    FullPipeline {
        /// Source ID to process
        #[arg(long)]
        source_id: String,
        /// Bypass cadence
        #[arg(long)]
        bypass_cadence: bool,
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
            
            println!("❌ Ingester command temporarily disabled due to compilation issues");
            println!("💡 Use the full-pipeline command instead: cargo run --bin sms-scraper -- full-pipeline --source-id {}", apis);
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
    }
    
    Ok(())
}
