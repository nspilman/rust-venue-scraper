use clap::{Parser, Subcommand};
use tracing::{info, warn, error, debug};

mod apis;
mod carpenter;
mod config;
mod constants;
mod error;
mod logging;
mod pipeline;
mod storage;
mod types;

use crate::apis::blue_moon::BlueMoonCrawler;
use crate::apis::sea_monster::SeaMonsterCrawler;
use crate::carpenter::Carpenter;
use crate::pipeline::Pipeline;
use crate::storage::{InMemoryStorage, Storage};
use crate::types::EventApi;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "sms_scraper")]
#[command(about = "Seattle Music Scene event data scraper")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the data ingestion process
    Ingester {
        /// Specific APIs to run (comma-separated). Available: blue_moon, sea_monster
        #[arg(long)]
        apis: Option<String>,
    },
    /// Run the data processing/cleaning
    Carpenter {
        /// Specific APIs to process (comma-separated) 
        #[arg(long)]
        apis: Option<String>,
        /// Process all data, not just unprocessed
        #[arg(long)]
        process_all: bool,
    },
    /// Run both ingester and carpenter sequentially
    Run {
        /// Specific APIs to run (comma-separated)
        #[arg(long)]
        apis: Option<String>,
    },
}

fn create_api(api_name: &str) -> Option<Box<dyn EventApi>> {
match api_name {
        constants::BLUE_MOON_API => Some(Box::new(BlueMoonCrawler::new())),
        constants::SEA_MONSTER_API => Some(Box::new(SeaMonsterCrawler::new())),
        _ => None,
    }
}

async fn run_apis(api_names: &[String], output_dir: &str, storage: Arc<dyn Storage>) -> Result<(), Box<dyn std::error::Error>> {
    for api_name in api_names {
        let span = tracing::info_span!("Running API", api = %api_name);
        let _enter = span.enter();
        
        if let Some(crawler) = create_api(api_name) {
            info!("Starting pipeline");
            match Pipeline::run_for_api_with_storage(crawler, output_dir, storage.clone()).await {
                Ok(result) => {
                    info!("Pipeline finished");
                    println!("\n📊 Pipeline Results for {}:", api_name);
                    println!("   Total events: {}", result.total_events);
                    println!("   Processed: {}", result.processed_events);
                    println!("   Skipped: {}", result.skipped_events);
                    println!("   Errors: {}", result.errors.len());
                    println!("   Output file: {}", result.output_file);
                    
                    if !result.errors.is_empty() {
                        warn!("{} errors encountered during pipeline run", result.errors.len());
                        println!("\n⚠️  Errors encountered:");
                        for error in &result.errors {
                            println!("   - {}", error);
                        }
                    }
                }
                Err(e) => {
                    error!("Pipeline failed: {}", e);
                }
            }
        } else {
            warn!("Unknown API specified");
            println!("⚠️  Unknown API: {}", api_name);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    logging::init_logging();
    
    let cli = Cli::parse();

    let output_dir = "output";
    
    match cli.command {
        Commands::Ingester { apis } => {
            println!("🔄 Running ingester pipeline...");
            
            let api_names = if let Some(api_list) = apis {
                api_list.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                vec![constants::BLUE_MOON_API.to_string()] // Default
            };
            
            let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
            run_apis(&api_names, output_dir, storage).await?;
        }
        Commands::Carpenter { apis, process_all } => {
            println!("🔨 Running carpenter pipeline...");
            
            let api_names = if let Some(api_list) = apis {
                // Convert user-friendly API names to internal names
let mapped_names: Vec<String> = api_list.split(',')
                    .map(|s| s.trim())
                    .map(constants::api_name_to_internal)
                    .collect();
                Some(mapped_names)
            } else {
                None
            };
            
            let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
            let carpenter = Carpenter::new(storage);
            
            match carpenter.run(api_names, None, process_all).await {
                Ok(()) => {
                    println!("✅ Carpenter run completed successfully");
                }
                Err(e) => {
                    error!("Carpenter run failed: {}", e);
                    println!("❌ Carpenter run failed: {}", e);
                }
            }
        }
        Commands::Run { apis } => {
            println!("🚀 Running full pipeline (ingester + carpenter)...");
            
            let api_names = if let Some(api_list) = apis {
                api_list.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                vec![constants::BLUE_MOON_API.to_string()] // Default
            };
            
            let storage: Arc<dyn Storage> = Arc::new(InMemoryStorage::new());
            
            // Step 1: Run ingester
            println!("\n📥 Step 1: Running ingester...");
            run_apis(&api_names, output_dir, storage.clone()).await?;
            
            // Step 2: Run carpenter
            println!("\n🔨 Step 2: Running carpenter...");
            let carpenter = Carpenter::new(storage);
            
            // Convert api names to the format expected by carpenter
let carpenter_api_names: Vec<String> = api_names.iter()
                .map(|name| constants::api_name_to_internal(name))
                .collect();
            
            match carpenter.run(Some(carpenter_api_names), None, false).await {
                Ok(()) => {
                    println!("✅ Full pipeline completed successfully!");
                }
                Err(e) => {
                    error!("Carpenter run failed: {}", e);
                    println!("❌ Carpenter run failed: {}", e);
                }
            }
        }
    }
    Ok(())
}
