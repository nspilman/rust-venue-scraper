use clap::{Parser, Subcommand};
use tracing::{error, info, warn};

mod apis;
mod carpenter;
mod constants;
mod db;
mod error;
mod graphql;
mod logging;
mod pipeline;
mod server;
mod storage;
mod types;

use crate::apis::blue_moon::BlueMoonCrawler;
use crate::apis::darrells_tavern::DarrellsTavernCrawler;
use crate::apis::sea_monster::SeaMonsterCrawler;
use crate::carpenter::Carpenter;
use crate::db::DatabaseManager;
use crate::pipeline::Pipeline;
use crate::storage::{DatabaseStorage, InMemoryStorage, Storage};
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
        /// Specific APIs to run (comma-separated). Available: blue_moon, sea_monster, darrells_tavern
        #[arg(long)]
        apis: Option<String>,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
    },
    /// Run the data processing/cleaning
    Carpenter {
        /// Specific APIs to process (comma-separated)
        #[arg(long)]
        apis: Option<String>,
        /// Process all data, not just unprocessed
        #[arg(long)]
        process_all: bool,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
    },
    /// Run both ingester and carpenter sequentially
    Run {
        /// Specific APIs to run (comma-separated)
        #[arg(long)]
        apis: Option<String>,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
    },
    /// Start the GraphQL API server
    Server {
        /// Port to run the server on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
    },
    /// Clear all data from the database (CAUTION: This will delete everything!)
    ClearDb,
}

fn create_api(api_name: &str) -> Option<Box<dyn EventApi>> {
    match api_name {
        constants::BLUE_MOON_API => Some(Box::new(BlueMoonCrawler::new())),
        constants::SEA_MONSTER_API => Some(Box::new(SeaMonsterCrawler::new())),
        constants::DARRELLS_TAVERN_API => Some(Box::new(DarrellsTavernCrawler::new())),
        _ => None,
    }
}

async fn create_storage(
    use_database: bool,
) -> Result<Arc<dyn Storage>, Box<dyn std::error::Error>> {
    if use_database {
        dotenv::dotenv().ok(); // Load environment variables

        info!("Creating database storage connection...");
        let db_storage = DatabaseStorage::new().await
            .map_err(|e| format!("Failed to initialize database storage: {e}. Make sure LIBSQL_URL and LIBSQL_AUTH_TOKEN environment variables are set."))?;

        info!("✅ Database storage initialized successfully");
        Ok(Arc::new(db_storage))
    } else {
        info!("Using in-memory storage (data will not persist)");
        Ok(Arc::new(InMemoryStorage::new()))
    }
}

async fn run_apis(
    api_names: &[String],
    output_dir: &str,
    storage: Arc<dyn Storage>,
) -> Result<(), Box<dyn std::error::Error>> {
    for api_name in api_names {
        let span = tracing::info_span!("Running API", api = %api_name);
        let _enter = span.enter();

        if let Some(crawler) = create_api(api_name) {
            info!("Starting pipeline");
            match Pipeline::run_for_api_with_storage(crawler, output_dir, storage.clone()).await {
                Ok(result) => {
                    info!("Pipeline finished");
                    println!("\n📊 Pipeline Results for {api_name}:");
                    println!("   Total events: {}", result.total_events);
                    println!("   Processed: {}", result.processed_events);
                    println!("   Skipped: {}", result.skipped_events);
                    println!("   Errors: {}", result.errors.len());
                    println!("   Output file: {}", result.output_file);

                    if !result.errors.is_empty() {
                        warn!(
                            "{} errors encountered during pipeline run",
                            result.errors.len()
                        );
                        println!("\n⚠️  Errors encountered:");
                        for error in &result.errors {
                            println!("   - {error}");
                        }
                    }
                }
                Err(e) => {
                    error!("Pipeline failed: {}", e);
                }
            }
        } else {
            warn!("Unknown API specified");
            println!("⚠️  Unknown API: {api_name}");
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
        Commands::Ingester { apis, use_database } => {
            println!("🔄 Running ingester pipeline...");

            let api_names: Vec<String> = if let Some(api_list) = apis {
                api_list.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                // Default to all supported APIs
                constants::get_supported_apis()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            };

            let storage = create_storage(use_database).await?;
            run_apis(&api_names, output_dir, storage).await?;
        }
        Commands::Carpenter {
            apis,
            process_all,
            use_database,
        } => {
            println!("🔨 Running carpenter pipeline...");

            let api_names = if let Some(api_list) = apis {
                // Convert user-friendly API names to internal names
                let mapped_names: Vec<String> = api_list
                    .split(',')
                    .map(|s| s.trim())
                    .map(constants::api_name_to_internal)
                    .collect();
                Some(mapped_names)
            } else {
                None
            };

            let storage = create_storage(use_database).await?;
            let carpenter = Carpenter::new(storage);

            match carpenter.run(api_names, None, process_all).await {
                Ok(()) => {
                    println!("✅ Carpenter run completed successfully");
                }
                Err(e) => {
                    error!("Carpenter run failed: {}", e);
                    println!("❌ Carpenter run failed: {e}");
                }
            }
        }
        Commands::Run { apis, use_database } => {
            println!("🚀 Running full pipeline (ingester + carpenter)...");

            let api_names: Vec<String> = if let Some(api_list) = apis {
                api_list.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                // Default to all supported APIs
                constants::get_supported_apis()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            };

            let storage = create_storage(use_database).await?;

            // Step 1: Run ingester
            println!("\n📥 Step 1: Running ingester...");
            run_apis(&api_names, output_dir, storage.clone()).await?;

            // Step 2: Run carpenter
            println!("\n🔨 Step 2: Running carpenter...");
            let carpenter = Carpenter::new(storage);

            // Convert api names to the format expected by carpenter
            let carpenter_api_names: Vec<String> = api_names
                .iter()
                .map(|name| constants::api_name_to_internal(name))
                .collect();

            match carpenter.run(Some(carpenter_api_names), None, false).await {
                Ok(()) => {
                    println!("✅ Full pipeline completed successfully!");
                }
                Err(e) => {
                    error!("Carpenter run failed: {}", e);
                    println!("❌ Carpenter run failed: {e}");
                }
            }
        }
        Commands::Server { port, use_database } => {
            println!("🚀 Starting GraphQL API server on port {port}...");

            let storage = create_storage(use_database).await?;

            println!("📡 Server endpoints:");
            println!("   GraphQL API: http://localhost:{port}/graphql");
            println!("   GraphiQL UI: http://localhost:{port}/graphiql");
            println!("   Playground UI: http://localhost:{port}/playground");
            println!("   Health check: http://localhost:{port}/health");
            println!();

            if use_database {
                println!("💾 Using database storage");
            } else {
                println!("🧠 Using in-memory storage (data will not persist)");
            }
            println!();

            match server::start_server(storage, port).await {
                Ok(()) => {
                    println!("✅ Server started successfully");
                }
                Err(e) => {
                    error!("Server failed to start: {}", e);
                    println!("❌ Server failed to start: {e}");
                }
            }
        }
        Commands::ClearDb => {
            println!("🗑️  Clearing all data from the database...");
            println!("⚠️  WARNING: This will permanently delete all data!");

            // Load environment variables
            dotenv::dotenv().ok();

            let db_manager = DatabaseManager::new().await
                .map_err(|e| format!("Failed to connect to database: {e}. Make sure LIBSQL_URL and LIBSQL_AUTH_TOKEN environment variables are set."))?;

            match db_manager.clear_all_data().await {
                Ok(()) => {
                    println!("✅ Database cleared successfully!");
                }
                Err(e) => {
                    error!("Failed to clear database: {}", e);
                    println!("❌ Failed to clear database: {e}");
                }
            }
        }
    }
    Ok(())
}
