use clap::{Parser, Subcommand};
use tracing::{error, info, warn};

// Import ALL modules for the full scraper
use sms_scraper::*;

// Re-export the original main logic from main.rs for the full scraper
// This will include all scraper APIs and heavy dependencies

#[derive(Parser)]
#[command(name = "scraper_full")]
#[command(about = "Full SMS scraper with all crawlers and processing")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Re-use the exact same Commands enum from main.rs
#[derive(Subcommand)]
enum Commands {
    /// Run data ingestion for specified APIs
    Ingester {
        /// Comma-separated list of APIs to run (e.g., blue_moon,sea_monster,darrells_tavern,kexp,barboza,neumos,conor_byrne)
        #[arg(long)]
        apis: String,
        /// Use database storage instead of in-memory
        #[arg(long)]
        use_database: bool,
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
    },
    /// Run the complete pipeline: Gateway → Parse → Normalize → Quality Gate → Enrich → Conflation → Catalog
    FullPipeline {
        /// Source ID to process (e.g., blue_moon, sea_monster, darrells_tavern, kexp, barboza, neumos)
        #[arg(long)]
        source_id: String,
        /// Use database storage instead of in-memory
        #[arg(long)]
        use_database: bool,
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    match cli.command {
        Commands::Ingester { apis, use_database, bypass_cadence } => {
            // Delegate to the original main.rs ingester logic
            sms_scraper::run_ingester(apis, use_database, bypass_cadence).await
        }
        Commands::FullPipeline { source_id, use_database, bypass_cadence } => {
            // Delegate to the original main.rs full pipeline logic
            sms_scraper::run_full_pipeline(source_id, use_database, bypass_cadence).await
        }
    }
}