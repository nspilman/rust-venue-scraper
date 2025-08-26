use clap::Parser;
use tracing::info;

// Only include modules needed for GraphQL server
use sms_scraper::{
    common,
    domain,
    graphql,
    observability,
    server,
};

#[cfg(feature = "db")]
use sms_scraper::{db::DatabaseManager, pipeline::storage::DatabaseStorage};
use sms_scraper::pipeline::storage::{InMemoryStorage, Storage};
use std::sync::Arc;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "graphql_server")]
#[command(about = "Lightweight GraphQL API server for SMS")]
#[command(version = "0.1.0")]
struct Cli {
    /// Port to run the server on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Metrics server address
    #[arg(long)]
    metrics_addr: Option<String>,

    /// Use database storage instead of in-memory
    #[arg(long)]
    use_database: bool,
}

async fn create_storage(use_database: bool) -> Result<Arc<dyn Storage>> {
    if use_database {
        #[cfg(feature = "db")]
        {
            info!("Initializing database storage...");
            let db_manager = DatabaseManager::new().await?;
            db_manager.run_migrations().await?;
            let storage = Arc::new(DatabaseStorage::new(db_manager).await?);
            info!("Database storage initialized successfully");
            Ok(storage)
        }
        #[cfg(not(feature = "db"))]
        {
            anyhow::bail!("Database feature not enabled. Rebuild with --features db");
        }
    } else {
        info!("Using in-memory storage");
        Ok(Arc::new(InMemoryStorage::new()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting lightweight GraphQL API server on port {}...", cli.port);

    // Initialize metrics with server address
    if let Some(addr) = cli.metrics_addr.as_deref() {
        std::env::set_var("SMS_METRICS_ADDR", addr);
    }
    sms_scraper::observability::metrics::init().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to initialize metrics: {}", e);
    });

    let storage = create_storage(cli.use_database).await?;

    println!("📡 Server endpoints:");
    println!("   GraphQL API: http://localhost:{}/graphql", cli.port);
    let maddr = std::env::var("SMS_METRICS_ADDR").unwrap_or_else(|_| "127.0.0.1:9464".to_string());
    println!("   Metrics: http://{}/metrics", maddr);
    println!("   GraphiQL UI: http://localhost:{}/graphiql", cli.port);
    println!("   Playground UI: http://localhost:{}/playground", cli.port);
    println!("   Health check: http://localhost:{}/health", cli.port);
    println!();

    if cli.use_database {
        println!("💾 Using database storage");
    } else {
        println!("🧠 Using in-memory storage (data will not persist)");
    }
    println!();

    match server::start_server(storage, cli.port).await {
        Ok(()) => {
            println!("✅ Server started successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Server failed to start: {e}");
            Err(e)
        }
    }
}