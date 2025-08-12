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

mod gateway;
mod envelope;
mod registry;
mod idempotency;
mod ingest_log_reader;
mod parser;
mod ingest_meta;
mod rate_limiter;
mod ingest_common;

use crate::apis::blue_moon::BlueMoonCrawler;
use crate::apis::darrells_tavern::DarrellsTavernCrawler;
use crate::apis::sea_monster::SeaMonsterCrawler;
use crate::carpenter::Carpenter;
use crate::db::DatabaseManager;
use crate::pipeline::Pipeline;
use crate::storage::{DatabaseStorage, InMemoryStorage, Storage};
use crate::types::EventApi;
use std::sync::Arc;
use std::path::PathBuf;

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
    Read { #[arg(long)] consumer: String, #[arg(long, default_value_t = 100)] max: usize },
    Ack { #[arg(long)] consumer: String, #[arg(long)] envelope_id: String },
    Status { #[arg(long)] consumer: String },
    Resolve { #[arg(long)] envelope_id: String },
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
        /// Bypass cadence (fetch even if fetched within the last interval)
        #[arg(long)]
        bypass_cadence: bool,
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
    /// One-off: fetch bytes for a source per registry, build envelope, persist CAS + envelope locally
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
    /// Clear all data from the database (CAUTION: This will delete everything!)
    ClearDb,
}

fn data_root_path_from_arg(data_root: &str) -> PathBuf {
    // If data_root is absolute, use it; otherwise anchor to project dir
    let p = PathBuf::from(data_root);
    if p.is_absolute() { p } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p) }
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
    // Initialize logging and load .env
    logging::init_logging();
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    let output_dir = "output";

    match cli.command {
        Commands::Ingester { apis, use_database, bypass_cadence } => {
            println!("🔄 Running ingester pipeline...");
            if bypass_cadence { std::env::set_var("SMS_BYPASS_CADENCE", "1"); }

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
        Commands::Run { apis, use_database, bypass_cadence } => {
            println!("🚀 Running full pipeline (ingester + carpenter)...");
            if bypass_cadence { std::env::set_var("SMS_BYPASS_CADENCE", "1"); }

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
        Commands::GatewayOnce { source_id, data_root, bypass_cadence } => {
            use crate::envelope::{ChecksumMeta, EnvelopeSubmissionV1, LegalMeta, PayloadMeta, RequestMeta, TimingMeta};
            use crate::gateway::Gateway;
            use crate::idempotency::compute_idempotency_key;
            use crate::registry::load_source_spec;
            use chrono::Utc;
            use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};

            let source = source_id.unwrap_or_else(|| constants::BLUE_MOON_API.to_string());
            if bypass_cadence { std::env::set_var("SMS_BYPASS_CADENCE", "1"); }
            let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let reg_path = base.join("registry/sources").join(format!("{}.json", source));
            println!("📘 Loading registry entry from {}", reg_path.to_string_lossy());
            let spec = load_source_spec(&reg_path).map_err(|e| format!("Failed to load registry: {e}"))?;
            if !spec.enabled {
                println!("⛔ Source is disabled in registry");
                return Ok(());
            }
            let ep = spec.endpoints.first().ok_or("No endpoint in registry")?;
            println!("🌐 Fetching {} {}", ep.method, ep.url);

            // Enforce very low cadence per source (twice a day), unless bypassed for development
            {
                let bypass = std::env::var("SMS_BYPASS_CADENCE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
                if !bypass {
                    let meta = crate::ingest_meta::IngestMeta::open_at_root(data_root_path_from_arg(&data_root))?;
                    let now = chrono::Utc::now().timestamp();
                    let min_interval_secs: i64 = 12 * 60 * 60; // 12 hours
                    if let Some(last) = meta.get_last_fetched_at(&spec.source_id)? {
                        let since = now - last;
                        if since < min_interval_secs {
                            let remain = min_interval_secs - since;
                            println!("⏳ Skipping fetch for '{}' to respect cadence ({}h remaining)", spec.source_id, (remain as f64 / 3600.0).ceil() as i64);
                            return Ok(());
                        }
                    }
                }
            }

            // Build per-source limiter from registry specs (optional; complements cadence)
            let rl = crate::rate_limiter::RateLimiter::new(crate::rate_limiter::Limits {
                requests_per_min: spec.rate_limits.requests_per_min,
                bytes_per_min: spec.rate_limits.bytes_per_min,
                concurrency: spec.rate_limits.concurrency.map(|c| c.max(1)),
            });

            let client = reqwest::Client::new();
            // Acquire for the request; we don't know size yet, so do RPM/concurrency before send
            rl.acquire(0).await;
            let resp = client.get(&ep.url).send().await?;
            let status = resp.status().as_u16();
            let headers = resp.headers().clone();
            let bytes = resp.bytes().await?;
            let payload = bytes.to_vec();

            // Account for bytes after we know the size
            rl.acquire(payload.len() as u64).await;

            let content_type = headers
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream").to_string();
            let content_length: u64 = headers
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(payload.len() as u64);
            let etag = headers.get(ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
            let last_modified = headers
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Safety checks per registry content section
            if content_length > spec.content.max_payload_size_bytes {
                return Err(format!("Payload too large: {} > {}", content_length, spec.content.max_payload_size_bytes).into());
            }
            let content_type_base = content_type.split(';').next().unwrap_or("").trim().to_string();
            if !spec.content.allowed_mime_types.iter().any(|m| m == &content_type_base) {
                return Err(format!(
                    "MIME '{}' not in allow-list {:?}",
                    content_type, spec.content.allowed_mime_types
                ).into());
            }


            // Compute checksum and idempotency key
            let sha_hex = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&payload);
                hex::encode(h.finalize())
            };
            let idk = compute_idempotency_key(&spec.source_id, &ep.url, etag.as_deref(), last_modified.as_deref(), &sha_hex);

            // Build envelope
            let env = EnvelopeSubmissionV1 {
                envelope_version: "1.0.0".to_string(),
                source_id: spec.source_id.clone(),
                idempotency_key: idk,
                payload_meta: PayloadMeta { mime_type: content_type, size_bytes: content_length, checksum: ChecksumMeta { sha256: sha_hex } },
                request: RequestMeta { url: ep.url.clone(), method: ep.method.clone(), status: Some(status), etag, last_modified },
                timing: TimingMeta { fetched_at: Utc::now(), gateway_received_at: None },
                legal: LegalMeta { license_id: spec.policy.license_id.clone() },
            };

            // Accept via gateway shim
            let gw = Gateway::new(data_root_path_from_arg(&data_root));
            let stamped = gw.accept(env, &payload).map_err(|e| format!("Gateway accept failed: {e}"))?;

            // Update cadence marker
            {
                let meta = crate::ingest_meta::IngestMeta::open_at_root(data_root_path_from_arg(&data_root))?;
                let now = chrono::Utc::now().timestamp();
                let _ = meta.set_last_fetched_at(&stamped.envelope.source_id, now);
            }

            println!("✅ Accepted envelope {} with payload {}", stamped.envelope_id, stamped.payload_ref);
            println!("📄 Ingest log: {}/ingest_log/ingest.ndjson", data_root_path_from_arg(&data_root).display());
            println!("📦 CAS root: {}/cas", data_root_path_from_arg(&data_root).display());
        }
        Commands::IngestLog { cmd } => {
            use crate::ingest_log_reader::IngestLogReader;
            let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
            let reader = IngestLogReader::new(data_root);
            match cmd {
                IngestLogCmd::Read { consumer, max } => {
                    let (lines, last) = reader.read_next(&consumer, max)?;
                    for l in lines { println!("{}", l); }
                    if let Some(id) = last { eprintln!("last_envelope_id={}", id); }
                }
                IngestLogCmd::Ack { consumer, envelope_id } => {
                    let off = reader.ack_through(&consumer, &envelope_id)?;
                    println!("ack_ok consumer={} file={} offset={} envelope_id={}", consumer, off.file, off.byte_offset, off.envelope_id.unwrap_or_default());
                }
                IngestLogCmd::Status { consumer } => {
                    let (off, end, lag) = reader.status(&consumer)?;
                    println!("consumer={} file={} offset={} last_envelope_id={} end={} lag_bytes={}", consumer, off.file, off.byte_offset, off.envelope_id.unwrap_or_default(), end, lag);
                }
                IngestLogCmd::Resolve { envelope_id } => {
                    if let Some(line) = reader.find_envelope_by_id(&envelope_id)? {
                        let val: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
                        if let Some(pref) = val.get("payload_ref").and_then(|v| v.as_str()) {
                            if let Some(path) = reader.resolve_payload_path(pref) {
                                println!("{}", path.display());
                            } else {
                                println!("could_not_resolve_payload_path");
                            }
                        } else if let Some(env) = val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str()) {
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
        Commands::Parse { consumer, max, data_root, output, source_id } => {
            use crate::ingest_log_reader::IngestLogReader;
            use crate::parser::{ParsedRecord, Parser, WixCalendarV1Parser, WixWarmupV1Parser, DarrellsHtmlV1Parser};
            use crate::registry::load_source_spec;
            use std::fs::OpenOptions;
            use std::io::Write;
            use std::path::Path;
            use tracing::{debug, error, info, warn};

            info!("parser: starting parse run consumer={} data_root={} output={} max={}", consumer, data_root, output, max);
            let reader = IngestLogReader::new(data_root_path_from_arg(&data_root));
            let (lines, _last) = reader.read_next(&consumer, max)?;
            info!("parser: read {} log lines from ingest log", lines.len());
            if lines.is_empty() { println!("no_envelopes"); return Ok(()); }

            // Build per-run output filename with datetime prefix
            let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let base_out = Path::new(&output);
            let dir = base_out.parent().unwrap_or(Path::new("."));
            let file = base_out.file_name().unwrap_or_else(|| std::ffi::OsStr::new("parsed.ndjson"));
            let prefixed_path = dir.join(format!("{}_{}", ts, file.to_string_lossy()));
            // Ensure directory exists
            std::fs::create_dir_all(dir)?;
            let mut out = OpenOptions::new().create(true).write(true).truncate(true).open(&prefixed_path)?;

            let mut total_seen = 0usize;
            let mut total_filtered = 0usize;
            let mut total_written = 0usize;
            let mut total_empty_records = 0usize;

            for line in lines {
                total_seen += 1;
                let val: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => { warn!("parser: skipping invalid JSON line: {}", e); continue; }
                };
                let mut payload_ref_s = val.get("payload_ref").and_then(|v| v.as_str())
                    .or_else(|| val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();
                let envelope_id = val.get("envelope_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let src_id = val.get("envelope").and_then(|e| e.get("source_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();

                // If this line is a dedupe placeholder (no payload_ref) try to resolve the original envelope to get its payload_ref
                if payload_ref_s.is_empty() {
                    if let Some(dedupe_of) = val.get("dedupe_of").and_then(|v| v.as_str()) {
                        let rdr = IngestLogReader::new(data_root_path_from_arg(&data_root));
                        if let Ok(Some(orig_line)) = rdr.find_envelope_by_id(dedupe_of) {
                            if let Ok(orig_val) = serde_json::from_str::<serde_json::Value>(&orig_line) {
                                if let Some(pr) = orig_val.get("payload_ref").and_then(|v| v.as_str())
                                    .or_else(|| orig_val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str())) {
                                    info!("parser: resolved dedupe envelope_id={} to original {} with payload_ref present", envelope_id, dedupe_of);
                                    payload_ref_s = pr.to_string();
                                } else {
                                    warn!("parser: original dedupe_of={} has no payload_ref", dedupe_of);
                                }
                            }
                        } else {
                            warn!("parser: could not resolve dedupe_of={} for envelope_id={}", dedupe_of, envelope_id);
                        }
                    }
                }
                // If still empty, synthesize payload_ref from checksum in envelope metadata
                if payload_ref_s.is_empty() {
                    if let Some(sha) = val.get("envelope")
                        .and_then(|e| e.get("payload_meta"))
                        .and_then(|pm| pm.get("checksum"))
                        .and_then(|c| c.get("sha256"))
                        .and_then(|s| s.as_str()) {
                        payload_ref_s = format!("cas:sha256:{}", sha);
                        info!("parser: synthesized payload_ref from checksum for envelope_id={}", envelope_id);
                    }
                }

                if let Some(filter) = &source_id {
                    if src_id != *filter {
                        total_filtered += 1;
                        continue;
                    }
                }
                if payload_ref_s.is_empty() || src_id.is_empty() {
                    warn!("parser: skipping envelope with missing fields: envelope_id='{}' src_id='{}' payload_ref_present={} ", envelope_id, src_id, !payload_ref_s.is_empty());
                    continue;
                }

                // Resolve bytes from CAS (Supabase public URL if configured; else local path)
                let bytes = if (std::env::var("SUPABASE_URL").is_ok() || std::env::var("SUPABASE_PROJECT_REF").is_ok()) && std::env::var("SUPABASE_BUCKET").is_ok() {
                    let project_ref = std::env::var("SUPABASE_PROJECT_REF").ok();
                    let supabase_url = std::env::var("SUPABASE_URL").ok().or_else(|| project_ref.map(|r| format!("https://{}.supabase.co", r))).unwrap();
                    let bucket = std::env::var("SUPABASE_BUCKET").unwrap();
                    let prefix = std::env::var("SUPABASE_PREFIX").unwrap_or_default();

                    let hex = &payload_ref_s["cas:sha256:".len()..];
                    let key = if prefix.is_empty() {
                        format!("sha256/{}/{}/{}", &hex[0..2], &hex[2..4], hex)
                    } else {
                        format!("{}/sha256/{}/{}/{}", prefix.trim_end_matches('/'), &hex[0..2], &hex[2..4], hex)
                    };
                    let base = supabase_url.trim_end_matches('/');
                    let client = reqwest::Client::new();
                    // First try public URL
                    let public_url = format!("{}/storage/v1/object/public/{}/{}", base, bucket, key);
                    debug!("parser: fetching payload via supabase public_url for envelope_id={} src_id={} key={}", envelope_id, src_id, key);
                    let mut resp = client.get(public_url).send().await?;
                    if !resp.status().is_success() {
                        // Fallback to authenticated URL in case bucket isn't public
                        let auth_url = format!("{}/storage/v1/object/{}/{}", base, bucket, key);
                        if let Ok(key_hdr) = std::env::var("SUPABASE_SERVICE_ROLE_KEY").or_else(|_| std::env::var("SUPABASE_ANON_KEY")) {
                            debug!("parser: retrying supabase auth_url for envelope_id={} status={} ", envelope_id, resp.status().as_u16());
                            resp = client
                                .get(auth_url)
                                .header("Authorization", format!("Bearer {}", key_hdr))
                                .header("apikey", key_hdr.clone())
                                .send()
                                .await?;
                        }
                    }
                    if !resp.status().is_success() { error!("parser: fetch_bytes_failed envelope_id={} status={}", envelope_id, resp.status().as_u16()); return Err(format!("fetch_bytes_failed: {}", resp.status()).into()); }
                    let b = resp.bytes().await?.to_vec();
                    debug!("parser: fetched bytes via supabase envelope_id={} len={} ", envelope_id, b.len());
                    b
                } else {
                    if let Some(path) = reader.resolve_payload_path(&payload_ref_s) {
                        debug!("parser: reading payload from local CAS path for envelope_id={} path={} ", envelope_id, path.display());
                        std::fs::read(path)?
                    } else { warn!("parser: could not resolve payload path for envelope_id={} payload_ref={}", envelope_id, payload_ref_s); continue }
                };
                debug!("parser: resolved payload bytes for envelope_id={} len={}", envelope_id, bytes.len());

                // Choose parser by parse_plan_ref from registry
                let base = Path::new(env!("CARGO_MANIFEST_DIR"));
                let reg_path = base.join("registry/sources").join(format!("{}.json", src_id));
                let spec = load_source_spec(&reg_path)?;
                let plan = spec.parse_plan_ref.clone().unwrap_or_else(|| "parse_plan:wix_calendar_v1".to_string());
                info!("parser: parsing envelope_id={} src_id={} plan={} bytes={}", envelope_id, src_id, plan, bytes.len());
                let recs: Vec<ParsedRecord> = match spec.parse_plan_ref.as_deref() {
                    Some("parse_plan:wix_calendar_v1") | None => {
                        let p = WixCalendarV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string());
                        p.parse(&bytes)?
                    }
                    Some("parse_plan:wix_warmup_v1") => {
                        let p = WixWarmupV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string());
                        p.parse(&bytes)?
                    }
                    Some("parse_plan:darrells_html_v1") => {
                        let p = DarrellsHtmlV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string());
                        p.parse(&bytes)?
                    }
                    Some(other) => {
                        warn!("parser: skipping envelope {} with unsupported parse plan {}", envelope_id, other);
                        Vec::new()
                    }
                };

                if recs.is_empty() {
                    warn!("parser: parser produced 0 records for envelope_id={} src_id={} plan={}", envelope_id, src_id, plan);
                    total_empty_records += 1;
                } else {
                    debug!("parser: writing {} records for envelope_id={}", recs.len(), envelope_id);
                }

                for r in recs.clone() {
                    let line = serde_json::to_string(&r)?;
                    writeln!(out, "{}", line)?;
                }
                total_written += recs.len();
            }
            info!("parser: done. seen={} filtered_out={} empty_record_envelopes={} written_records={}", total_seen, total_filtered, total_empty_records, total_written);
            println!("parse_done -> {}", prefixed_path.to_string_lossy());
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
