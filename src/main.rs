use clap::{Parser, Subcommand};
use tracing::{error, info, warn};
// Import metrics macros from the external crate explicitly to avoid collision with local `metrics` module
// Removed unused import HttpClientPort

// Core modules
mod apis;
mod common;
#[cfg(feature = "db")]
mod db;
mod domain;
mod graphql;
mod observability;
mod pipeline;
mod server;

// Application and infrastructure layers
mod app;
mod infra;

// Architecture scaffolding (unreferenced for now)
#[allow(dead_code)]
mod architecture;

use crate::apis::blue_moon::BlueMoonCrawler;
use crate::apis::darrells_tavern::DarrellsTavernCrawler;
use crate::apis::sea_monster::SeaMonsterCrawler;
#[cfg(feature = "db")]
use crate::db::DatabaseManager;
use crate::pipeline::pipeline::Pipeline;
#[cfg(feature = "db")]
use crate::pipeline::storage::DatabaseStorage;
use crate::pipeline::storage::{InMemoryStorage, Storage};
use crate::common::types::EventApi;
use std::path::PathBuf;
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
enum IngestLogCmd {
    Read {
        #[arg(long)]
        consumer: String,
        #[arg(long, default_value_t = 100)]
        max: usize,
    },
    Ack {
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        envelope_id: String,
    },
    Status {
        #[arg(long)]
        consumer: String,
    },
    Resolve {
        #[arg(long)]
        envelope_id: String,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Parse envelopes from ingest log into neutral records
    Parse {
        /// Consumer name for offsets
        #[arg(long, default_value = "parser")]
        consumer: String,
        /// Max envelopes to process this run
        #[arg(long, default_value_t = 50)]
        max: usize,
        /// Data root (where ingest_log and local cas live)
        #[arg(long, default_value = "data")]
        data_root: String,
        /// Output NDJSON file for parsed records
        #[arg(long, default_value = "parsed.ndjson")]
        output: String,
        /// Optional: only parse envelopes for this source_id (e.g., blue_moon)
        #[arg(long)]
        source_id: Option<String>,
        /// Also run normalization step after parsing
        #[arg(long)]
        normalize: bool,
        /// Also run quality gate assessment after normalization (requires --normalize)
        #[arg(long)]
        quality_gate: bool,
    },
    /// Run the data ingestion process
    Ingester {
        /// Specific APIs to run (comma-separated). Available: blue_moon, sea_monster, darrells_tavern
        #[arg(long)]
        apis: Option<String>,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
    },
    /// Start the GraphQL API server
    Server {
        /// Port to run the server on
        #[arg(long, default_value = "8080")]
        port: u16,
        /// Metrics bind address (host:port). Default 127.0.0.1:9464
        #[arg(long)]
        metrics_addr: Option<String>,
        /// Use database storage instead of in-memory (requires LIBSQL_URL and LIBSQL_AUTH_TOKEN env vars)
        #[arg(long)]
        use_database: bool,
    },
    /// One-off: fetch bytes for a source per registry, build envelope, persist CAS + envelope locally
    #[command(alias = "GatewayOnce")]
    GatewayOnce {
        /// Source id to ingest (defaults to blue_moon)
        #[arg(long)]
        source_id: Option<String>,
        /// Root data directory for CAS and ingest log (defaults to ./data)
        #[arg(long, default_value = "data")]
        data_root: String,
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
    },
    /// Ingest log utilities
    IngestLog {
        #[command(subcommand)]
        cmd: IngestLogCmd,
    },
    /// Architectural demo: ingest a single source via registry (ports/adapters)
    ArchIngestOnce {
        /// Source id to ingest (e.g., blue_moon)
        #[arg(long)]
        source_id: String,
        /// Data root (for CAS and ingest log)
        #[arg(long, default_value = "data")]
        data_root: String,
    },
    /// Clear all data from the database (CAUTION: This will delete everything!)
    ClearDb,
}

fn data_root_path_from_arg(data_root: &str) -> PathBuf {
    // If data_root is absolute, use it; otherwise anchor to project dir
    let p = PathBuf::from(data_root);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
    }
}

fn create_api(api_name: &str) -> Option<Box<dyn EventApi>> {
    use crate::common::constants;
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
        #[cfg(feature = "db")]
        {
            dotenv::dotenv().ok(); // Load environment variables
            info!("Creating database storage connection...");
            let db_storage = DatabaseStorage::new().await
                .map_err(|e| format!("Failed to initialize database storage: {e}. Make sure LIBSQL_URL and LIBSQL_AUTH_TOKEN environment variables are set."))?;
            info!("✅ Database storage initialized successfully");
            return Ok(Arc::new(db_storage));
        }
        #[cfg(not(feature = "db"))]
        {
            warn!("Database feature not enabled at build time; falling back to in-memory storage");
            return Ok(Arc::new(InMemoryStorage::new()));
        }
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

            // Metrics: mark run start and time the pipeline execution per API
            let started = std::time::Instant::now();
            // Record registry load
                    crate::observability::metrics::sources::registry_load_success();

            match Pipeline::run_for_api_with_storage(crawler, output_dir, storage.clone()).await {
                Ok(result) => {
                    // Metrics: record successful duration and outcome counts
                    let duration = started.elapsed().as_secs_f64();
                    crate::observability::metrics::parser::parse_success();
                    crate::observability::metrics::parser::duration(duration);
                    crate::observability::metrics::parser::records_extracted(result.processed_events as u64);
                    
                    if result.errors.len() > 0 {
                        crate::observability::metrics::parser::parse_error();
                    }

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
                    // Metrics: record failure
                    crate::observability::metrics::parser::parse_error();

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
    // Initialize logging and load .env
    observability::init_logging();
    // NOTE: Metrics exporter is only initialized for long-lived processes (Server command)
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    let output_dir = "output";

    match cli.command {
        Commands::Ingester {
            apis,
            use_database,
            bypass_cadence,
        } => {
            println!("🔄 Running ingester pipeline...");
            if bypass_cadence {
                std::env::set_var("SMS_BYPASS_CADENCE", "1");
            }
            // Initialize metrics system
            crate::observability::init().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize metrics: {}", e);
            });
            // Heartbeat to ensure snapshot is non-empty for short runs
            crate::observability::heartbeat();

            let api_names: Vec<String> = if let Some(api_list) = apis {
                api_list.split(',').map(|s| s.trim().to_string()).collect()
            } else {
                // Default to all supported APIs
                crate::common::constants::get_supported_apis()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            };

            let storage = create_storage(use_database).await?;
            run_apis(&api_names, output_dir, storage).await?;
            
            // Push all collected metrics to Pushgateway before exit
            info!("Pushing metrics to Pushgateway...");
            if let Err(e) = crate::observability::metrics::push_all_metrics().await {
                warn!("Failed to push metrics: {}", e);
            } else {
                info!("Successfully pushed metrics to Pushgateway");
            }
        }
        Commands::Server { port, metrics_addr, use_database } => {
            println!("🚀 Starting GraphQL API server on port {port}...");

            // Initialize metrics with server address
            if let Some(addr) = metrics_addr.as_deref() {
                std::env::set_var("SMS_METRICS_ADDR", addr);
            }
            crate::observability::metrics::init().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize metrics: {}", e);
            });

            let storage = create_storage(use_database).await?;

            println!("📡 Server endpoints:");
            println!("   GraphQL API: http://localhost:{port}/graphql");
            let maddr = std::env::var("SMS_METRICS_ADDR").unwrap_or_else(|_| "127.0.0.1:9464".to_string());
            println!("   Metrics: http://{}/metrics", maddr);
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
        Commands::ArchIngestOnce { source_id, data_root } => {
            use crate::architecture::application::UseCases;
            use crate::architecture::infrastructure::{LocalCasAndLog, MetricsForwarder, ReqwestHttpClient, UtcClock};

            // Resolve endpoint from registry by source_id
            let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let reg_path = base.join("registry/sources").join(format!("{}.json", source_id));
            let spec = crate::pipeline::ingestion::registry::load_source_spec(&reg_path)
                .map_err(|e| format!("Failed to load registry: {e}"))?;
            if !spec.enabled {
                println!("⛔ Source is disabled in registry");
                return Ok(());
            }
            let ep = spec.endpoints.first().ok_or("No endpoint in registry")?;
            let url = &ep.url;
            let method = &ep.method;

            println!("🔧 Arch demo: ingesting {} {} (source_id={})", method, url, spec.source_id);
            let http = std::sync::Arc::new(ReqwestHttpClient);
            let store = std::sync::Arc::new(LocalCasAndLog::new(data_root_path_from_arg(&data_root)));
let _metrics = std::sync::Arc::new(MetricsForwarder);
            let clock = std::sync::Arc::new(UtcClock);

            let uc = UseCases::new(store, http, _metrics, clock);
            match uc.ingest_source_once(url, method).await {
                Ok(res) => {
                    println!("✅ envelope_id={} payload_ref={} bytes_written={}", res.envelope_id, res.payload_ref, res.bytes_written);
                }
                Err(e) => {
                    eprintln!("❌ ingest_failed: {}", e);
                }
            }
        }
        Commands::GatewayOnce {
            source_id,
            data_root,
            bypass_cadence,
        } => {
            use crate::infra::{http_client::ReqwestHttp, rate_limiter_adapter::RateLimiterAdapter, cadence_adapter::IngestMetaCadence, gateway_adapter::GatewayAdapter};
            use crate::app::ingest_use_case::IngestUseCase;
            
            use crate::pipeline::ingestion::registry::load_source_spec;

            let source = source_id.unwrap_or_else(|| crate::common::constants::BLUE_MOON_API.to_string());
            
            // Initialize metrics with push gateway support
            std::env::set_var("SMS_PUSHGATEWAY_URL", "http://localhost:9091");
            crate::observability::metrics::init_with_push_options(Some("sms_scraper"), Some(&source))
                .unwrap_or_else(|e| {
                    eprintln!("Warning: Failed to initialize metrics: {}", e);
                });
            crate::observability::heartbeat();
            if bypass_cadence { std::env::set_var("SMS_BYPASS_CADENCE", "1"); }
            let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let reg_path = base.join("registry/sources").join(format!("{}.json", source));
            println!("📘 Loading registry entry from {}", reg_path.to_string_lossy());
            let spec = load_source_spec(&reg_path).map_err(|e| format!("Failed to load registry: {e}"))?;
            if !spec.enabled { println!("⛔ Source is disabled in registry"); return Ok(()); }
            let ep = spec.endpoints.first().ok_or("No endpoint in registry")?;

            // Wire adapters
            let rl = crate::pipeline::ingestion::rate_limiter::RateLimiter::new(crate::pipeline::ingestion::rate_limiter::Limits {
                requests_per_min: spec.rate_limits.requests_per_min,
                bytes_per_min: spec.rate_limits.bytes_per_min,
                concurrency: spec.rate_limits.concurrency.map(|c| c.max(1)),
            });
            let usecase = IngestUseCase::new(
                Box::new(RateLimiterAdapter(rl)),
                Box::new(IngestMetaCadence),
                Box::new(ReqwestHttp),
                Box::new(GatewayAdapter { root: data_root_path_from_arg(&data_root) }),
            );

            // Track timing
            let start_time = std::time::Instant::now();
            
            // Run once
            match usecase.ingest_once(
                &spec.source_id,
                &ep.url,
                &ep.method,
                spec.content.max_payload_size_bytes,
                &spec.content.allowed_mime_types,
                &spec.policy.license_id,
            ).await {
                Ok((envelope_id, payload_ref, bytes)) => {
                    let duration_secs = start_time.elapsed().as_secs_f64();
                    println!("✅ Accepted envelope {} with payload {} ({} bytes in {:.2}s)", 
                        envelope_id, payload_ref, bytes, duration_secs);
                    println!("📄 Ingest log: {}/ingest_log/ingest.ndjson", data_root_path_from_arg(&data_root).display());
                    println!("📦 CAS root: {}/cas", data_root_path_from_arg(&data_root).display());
                    
                    // Push detailed metrics to Pushgateway
                    // Push ALL collected metrics to Pushgateway
                    info!("Pushing all metrics to Pushgateway...");
                    if let Err(e) = crate::observability::metrics::push_all_metrics_with_instance(&spec.source_id).await {
                        warn!("Failed to push metrics: {}", e);
                    } else {
                        info!("Successfully pushed all metrics to Pushgateway");
                    }
                }
                Err(e) => {
                    let duration_secs = start_time.elapsed().as_secs_f64();
                    println!("❌ ingest_failed after {:.2}s: {}", duration_secs, e);
                    
                    // Push failure metrics
                    info!("Pushing failure metrics to Pushgateway...");
                    if let Err(e) = crate::observability::metrics::push_all_metrics_with_instance(&spec.source_id).await {
                        warn!("Failed to push metrics: {}", e);
                    }
                }
            }
        }
        Commands::IngestLog { cmd } => {
            use crate::pipeline::ingestion::ingest_log_reader::IngestLogReader;
            let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
            let reader = IngestLogReader::new(data_root);
            match cmd {
                IngestLogCmd::Read { consumer, max } => {
                    let (lines, last) = reader.read_next(&consumer, max)?;
                    for l in lines {
                        println!("{}", l);
                    }
                    if let Some(id) = last {
                        eprintln!("last_envelope_id={}", id);
                    }
                }
                IngestLogCmd::Ack {
                    consumer,
                    envelope_id,
                } => {
                    let off = reader.ack_through(&consumer, &envelope_id)?;
                    println!(
                        "ack_ok consumer={} file={} offset={} envelope_id={}",
                        consumer,
                        off.file,
                        off.byte_offset,
                        off.envelope_id.unwrap_or_default()
                    );
                }
                IngestLogCmd::Status { consumer } => {
                    let (off, end, lag) = reader.status(&consumer)?;
                    println!(
                        "consumer={} file={} offset={} last_envelope_id={} end={} lag_bytes={}",
                        consumer,
                        off.file,
                        off.byte_offset,
                        off.envelope_id.unwrap_or_default(),
                        end,
                        lag
                    );
                }
                IngestLogCmd::Resolve { envelope_id } => {
                    if let Some(line) = reader.find_envelope_by_id(&envelope_id)? {
                        let val: serde_json::Value =
                            serde_json::from_str(&line).unwrap_or_default();
                        if let Some(pref) = val.get("payload_ref").and_then(|v| v.as_str()) {
                            if let Some(path) = reader.resolve_payload_path(pref) {
                                println!("{}", path.display());
                            } else {
                                println!("could_not_resolve_payload_path");
                            }
                        } else if let Some(env) = val
                            .get("envelope")
                            .and_then(|e| e.get("payload_ref"))
                            .and_then(|v| v.as_str())
                        {
                            if let Some(path) = reader.resolve_payload_path(env) {
                                println!("{}", path.display());
                            } else {
                                println!("could_not_resolve_payload_path");
                            }
                        } else {
                            println!("payload_ref_not_found");
                        }
                    } else {
                        println!("envelope_not_found");
                    }
                }
            }
        }
        Commands::Parse { consumer, max, data_root, output, source_id, normalize, quality_gate } => {
            // Initialize metrics system
            crate::observability::metrics::init().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize metrics: {}", e);
            });
            crate::observability::heartbeat();
            
            // Delegate to tasks::parse_run, which now uses a centralized ParseUseCase
            let params = crate::pipeline::tasks::ParseParams {
                consumer: Some(consumer),
                max: Some(max),
                data_root: Some(data_root),
                output: Some(output.clone()),
                source_id: source_id.clone(),
                normalize: Some(normalize),
                quality_gate: Some(quality_gate),
            };
            let storage = std::sync::Arc::new(InMemoryStorage::new()) as std::sync::Arc<dyn Storage>;
            match crate::pipeline::tasks::parse_run(storage, params).await {
                Ok(summary) => {
                    println!("parse_done -> {}", summary.output_file);
                    println!("seen={} filtered_out={} empty_record_envelopes={} written_records={}", summary.seen, summary.filtered_out, summary.empty_record_envelopes, summary.written_records);
                    
                    // Push metrics to Pushgateway
                    let instance = source_id.as_deref().unwrap_or("parser");
                    info!("Pushing parser metrics to Pushgateway for instance {}...", instance);
                    if let Err(e) = crate::observability::metrics::push_all_metrics_with_instance(instance).await {
                        warn!("Failed to push metrics: {}", e);
                    } else {
                        info!("Successfully pushed metrics to Pushgateway");
                    }
                }
                Err(e) => {
                    error!("Parse run failed: {}", e);
                    println!("❌ Parse run failed: {e}");
                    
                    // Still push metrics even on failure
                    let instance = source_id.as_deref().unwrap_or("parser");
                    if let Err(e) = crate::observability::metrics::push_all_metrics_with_instance(instance).await {
                        warn!("Failed to push metrics: {}", e);
                    }
                }
            }
        }
        Commands::ClearDb => {
            println!("🗑️  Clearing all data from the database...");
            println!("⚠️  WARNING: This will permanently delete all data!");
            #[cfg(feature = "db")]
            {
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
            #[cfg(not(feature = "db"))]
            {
                println!("Database feature not enabled at build time; nothing to clear.");
            }
        }
    }
    Ok(())
}
