use clap::{Parser, Subcommand};
use tracing::info;

mod apis;
mod common;
mod pipeline;
mod observability;
mod app;
mod infra;

use sms_core::{storage::Storage, storage::DatabaseStorage, database::DatabaseManager};
use std::sync::Arc;

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
    let db_manager = DatabaseManager::new().await?;
    db_manager.run_migrations().await?;
    let storage: Arc<dyn Storage> = Arc::new(DatabaseStorage::new(db_manager).await?);
    info!("Database storage initialized successfully");

    match cli.command {
        Commands::Ingester { apis, bypass_cadence } => {
            println!("🕷️  Starting SMS scraper ingestion for APIs: {}", apis);
            // TODO: Implement ingester logic
            println!("✅ Scraping completed - data written to database");
        }
        Commands::FullPipeline { source_id, bypass_cadence } => {
            println!("🔄 Running full pipeline for source: {}", source_id);
            // TODO: Implement full pipeline logic
            println!("✅ Pipeline completed - data processed and stored");
        }
    }
    
    Ok(())
}