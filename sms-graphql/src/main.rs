use clap::Parser;
use tracing::info;
use std::sync::Arc;

mod graphql;
mod server;

use sms_core::{storage::Storage, storage::DatabaseStorage, database::DatabaseManager};

#[derive(Parser)]
#[command(name = "sms-graphql")]
#[command(about = "Lightweight GraphQL API server for SMS")]
#[command(version = "0.1.0")]
struct Cli {
    /// Port to run the server on
    #[arg(short, long, default_value = "8080")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting SMS GraphQL API server on port {}...", cli.port);

    // Initialize database storage
    info!("Initializing database storage...");
    let db_manager = DatabaseManager::new().await?;
    db_manager.run_migrations().await?;
    let storage: Arc<dyn Storage> = Arc::new(DatabaseStorage::new().await?);
    info!("Database storage initialized successfully");

    println!("📡 Server endpoints:");
    println!("   GraphQL API: http://localhost:{}/graphql", cli.port);
    println!("   GraphiQL UI: http://localhost:{}/graphiql", cli.port);
    println!("   Health check: http://localhost:{}/health", cli.port);
    println!();

    // Start the server
    server::start_server(storage, cli.port).await?;
    
    Ok(())
}