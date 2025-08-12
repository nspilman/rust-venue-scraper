use crate::envelope::{ChecksumMeta, EnvelopeSubmissionV1, LegalMeta, PayloadMeta, RequestMeta, TimingMeta};
use crate::gateway::Gateway;
use crate::idempotency::compute_idempotency_key;
use crate::ingest_log_reader::IngestLogReader;
use crate::ingest_meta::IngestMeta;
use crate::parser::{ParsedRecord, Parser, WixCalendarV1Parser, WixWarmupV1Parser, DarrellsHtmlV1Parser};
use crate::registry::load_source_spec;
use crate::storage::{Storage};
use crate::constants;
use crate::error::ScraperError;
use crate::rate_limiter::{Limits, RateLimiter};
use chrono::Utc;
use metrics::{counter, histogram};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn, error};

fn data_root_path_from_arg(data_root: &str) -> PathBuf {
    let p = PathBuf::from(data_root);
    if p.is_absolute() { p } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p) }
}

#[derive(Debug, Deserialize)]
pub struct GatewayOnceParams {
    pub source_id: Option<String>,
    pub data_root: Option<String>,
    pub bypass_cadence: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct GatewayOnceResult {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_bytes: usize,
    pub ingest_log: String,
    pub cas_root: String,
}

pub async fn gateway_once(_storage: Arc<dyn Storage>, params: GatewayOnceParams) -> Result<GatewayOnceResult, Box<dyn std::error::Error>> {
    let source = params.source_id.unwrap_or_else(|| constants::BLUE_MOON_API.to_string());
    if params.bypass_cadence.unwrap_or(false) { std::env::set_var("SMS_BYPASS_CADENCE", "1"); }
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let reg_path = base.join("registry/sources").join(format!("{}.json", source));
    let spec = load_source_spec(&reg_path).map_err(|e| format!("Failed to load registry: {e}"))?;
    if !spec.enabled { return Err("Source is disabled".into()); }
    let ep = spec.endpoints.first().ok_or("No endpoint in registry")?;

    // Cadence check
    let data_root = data_root_path_from_arg(params.data_root.as_deref().unwrap_or("data"));
    {
        let c = counter!("sms_cadence_checks_total");
        c.increment(1);
    }
    let bypass = std::env::var("SMS_BYPASS_CADENCE").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
    if !bypass {
        let meta = IngestMeta::open_at_root(&data_root)?;
        let now = Utc::now().timestamp();
        let min_interval_secs: i64 = 12 * 60 * 60;
        if let Some(last) = meta.get_last_fetched_at(&spec.source_id)? {
            if now - last < min_interval_secs { return Err("cadence_skip: fetched within last 12h".into()); }
        }
    }

    // Rate limit and fetch
    let rl = RateLimiter::new(Limits {
        requests_per_min: spec.rate_limits.requests_per_min,
        bytes_per_min: spec.rate_limits.bytes_per_min,
        concurrency: spec.rate_limits.concurrency.map(|c| c.max(1)),
    });
    let client = reqwest::Client::new();
    rl.acquire(0).await;
    let t0 = std::time::Instant::now();
    let resp = client.get(&ep.url).send().await?;
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    let bytes = resp.bytes().await?.to_vec();
    rl.acquire(bytes.len() as u64).await;

let dur = t0.elapsed().as_secs_f64();
    println!("[metrics] record sms_fetch_duration_seconds={}s", dur);
    ::metrics::histogram!("sms_fetch_duration_seconds").record(dur);
    println!("[metrics] record sms_fetch_payload_bytes={}", bytes.len());
    ::metrics::histogram!("sms_fetch_payload_bytes").record(bytes.len() as f64);
    if (200..=299).contains(&status) { println!("[metrics] inc sms_fetch_success_total"); ::metrics::counter!("sms_fetch_success_total").increment(1); } else { println!("[metrics] inc sms_fetch_error_total"); ::metrics::counter!("sms_fetch_error_total").increment(1); }

    let content_type = headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
    let content_length: u64 = headers.get(CONTENT_LENGTH).and_then(|v| v.to_str().ok()).and_then(|s| s.parse().ok()).unwrap_or(bytes.len() as u64);
    let etag = headers.get(ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let last_modified = headers.get(LAST_MODIFIED).and_then(|v| v.to_str().ok()).map(|s| s.to_string());

    if content_length > spec.content.max_payload_size_bytes {
        return Err(format!("Payload too large: {} > {}", content_length, spec.content.max_payload_size_bytes).into());
    }
    let content_type_base = content_type.split(';').next().unwrap_or("").trim().to_string();
    if !spec.content.allowed_mime_types.iter().any(|m| m == &content_type_base) {
        return Err(format!("MIME '{}' not in allow-list {:?}", content_type, spec.content.allowed_mime_types).into());
    }

    let sha_hex = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&bytes);
        hex::encode(h.finalize())
    };
    let idk = compute_idempotency_key(&spec.source_id, &ep.url, etag.as_deref(), last_modified.as_deref(), &sha_hex);

    let env = EnvelopeSubmissionV1 {
        envelope_version: "1.0.0".to_string(),
        source_id: spec.source_id.clone(),
        idempotency_key: idk,
        payload_meta: PayloadMeta { mime_type: content_type, size_bytes: content_length, checksum: ChecksumMeta { sha256: sha_hex } },
        request: RequestMeta { url: ep.url.clone(), method: ep.method.clone(), status: Some(status), etag, last_modified },
        timing: TimingMeta { fetched_at: Utc::now(), gateway_received_at: None },
        legal: LegalMeta { license_id: spec.policy.license_id.clone() },
    };

    let gw = Gateway::new(data_root.clone());
    let stamped = gw.accept(env, &bytes)?;
    let _ = IngestMeta::open_at_root(&data_root)?.set_last_fetched_at(&stamped.envelope.source_id, Utc::now().timestamp());

    Ok(GatewayOnceResult {
        source_id: spec.source_id,
        envelope_id: stamped.envelope_id,
        payload_bytes: bytes.len(),
        ingest_log: data_root.join("ingest_log/ingest.ndjson").to_string_lossy().to_string(),
        cas_root: data_root.join("cas").to_string_lossy().to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct ParseParams {
    pub consumer: Option<String>,
    pub max: Option<usize>,
    pub data_root: Option<String>,
    pub output: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParseResultSummary {
    pub seen: usize,
    pub filtered_out: usize,
    pub empty_record_envelopes: usize,
    pub written_records: usize,
    pub output_file: String,
}

pub async fn parse_run(_storage: Arc<dyn Storage>, params: ParseParams) -> Result<ParseResultSummary, Box<dyn std::error::Error>> {
    let consumer = params.consumer.unwrap_or_else(|| "parser".to_string());
    let max = params.max.unwrap_or(50);
    let data_root_s = params.data_root.unwrap_or_else(|| "data".to_string());
    let output = params.output.unwrap_or_else(|| "parsed.ndjson".to_string());

    let reader = IngestLogReader::new(data_root_path_from_arg(&data_root_s));
    let (lines, _last) = reader.read_next(&consumer, max)?;
    info!("parser: read {} log lines from ingest log", lines.len());
println!("[metrics] inc sms_parse_runs_total");
    ::metrics::counter!("sms_parse_runs_total").increment(1);
    println!("[metrics] record sms_parse_loglines_per_run={} lines", lines.len());
    ::metrics::histogram!("sms_parse_loglines_per_run").record(lines.len() as f64);
    if lines.is_empty() { return Ok(ParseResultSummary{seen:0,filtered_out:0,empty_record_envelopes:0,written_records:0,output_file:"".to_string()}); }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let base_out = Path::new(&output);
    let dir = base_out.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let file = base_out.file_name().unwrap_or_else(|| std::ffi::OsStr::new("parsed.ndjson"));
    let prefixed_path = dir.join(format!("{}_{}", ts, file.to_string_lossy()));
    let mut out = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&prefixed_path)?;

    let mut total_seen = 0usize;
    let mut total_filtered = 0usize;
    let mut total_written = 0usize;
    let mut total_empty_records = 0usize;

    for line in lines {
        total_seen += 1;
        let val: serde_json::Value = match serde_json::from_str(&line) { Ok(v) => v, Err(e) => { warn!("parser: skipping invalid JSON line: {}", e); continue; } };
        let mut payload_ref_s = val.get("payload_ref").and_then(|v| v.as_str())
            .or_else(|| val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str()))
            .unwrap_or("").to_string();
        let envelope_id = val.get("envelope_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let src_id = val.get("envelope").and_then(|e| e.get("source_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();
        counter!("sms_parse_envelopes_seen_total").increment(1);

        if payload_ref_s.is_empty() {
            if let Some(dedupe_of) = val.get("dedupe_of").and_then(|v| v.as_str()) {
                let rdr = IngestLogReader::new(data_root_path_from_arg(&data_root_s));
                if let Ok(Some(orig_line)) = rdr.find_envelope_by_id(dedupe_of) {
                    if let Ok(orig_val) = serde_json::from_str::<serde_json::Value>(&orig_line) {
                        if let Some(pr) = orig_val.get("payload_ref").and_then(|v| v.as_str())
                            .or_else(|| orig_val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str())) {
                            info!("parser: resolved dedupe envelope_id={} to original {} with payload_ref present", envelope_id, dedupe_of);
                            payload_ref_s = pr.to_string();
                        } else { warn!("parser: original dedupe_of={} has no payload_ref", dedupe_of); }
                    }
                } else { warn!("parser: could not resolve dedupe_of={} for envelope_id={}", dedupe_of, envelope_id); }
            }
        }
        if payload_ref_s.is_empty() {
            if let Some(sha) = val.get("envelope").and_then(|e| e.get("payload_meta")).and_then(|pm| pm.get("checksum")).and_then(|c| c.get("sha256")).and_then(|s| s.as_str()) {
                payload_ref_s = format!("cas:sha256:{}", sha);
                info!("parser: synthesized payload_ref from checksum for envelope_id={}", envelope_id);
            }
        }
        if let Some(filter) = &params.source_id { if src_id != *filter { total_filtered += 1; counter!("sms_parse_envelopes_filtered_total").increment(1); continue; } }
        if payload_ref_s.is_empty() || src_id.is_empty() { warn!("parser: skipping envelope with missing fields: envelope_id='{}' src_id='{}' payload_ref_present={} ", envelope_id, src_id, !payload_ref_s.is_empty()); counter!("sms_parse_skipped_envelopes_total_missing_fields").increment(1); continue; }

        // Resolve bytes from local CAS or Supabase
        let bytes = if (std::env::var("SUPABASE_URL").is_ok() || std::env::var("SUPABASE_PROJECT_REF").is_ok()) && std::env::var("SUPABASE_BUCKET").is_ok() {
            let project_ref = std::env::var("SUPABASE_PROJECT_REF").ok();
            let supabase_url = std::env::var("SUPABASE_URL").ok().or_else(|| project_ref.map(|r| format!("https://{}.supabase.co", r))).unwrap();
            let bucket = std::env::var("SUPABASE_BUCKET").unwrap();
            let prefix = std::env::var("SUPABASE_PREFIX").unwrap_or_default();
            let hex = &payload_ref_s["cas:sha256:".len()..];
            let key = if prefix.is_empty() { format!("sha256/{}/{}/{}", &hex[0..2], &hex[2..4], hex) } else { format!("{}/sha256/{}/{}/{}", prefix.trim_end_matches('/'), &hex[0..2], &hex[2..4], hex) };
            let base = supabase_url.trim_end_matches('/');
            let client = reqwest::Client::new();
            let public_url = format!("{}/storage/v1/object/public/{}/{}", base, bucket, key);
            debug!("parser: fetching payload via supabase public_url for envelope_id={} src_id={} key={}", envelope_id, src_id, key);
            let mut resp = client.get(public_url).send().await?;
            if !resp.status().is_success() {
                let auth_url = format!("{}/storage/v1/object/{}/{}", base, bucket, key);
                if let Ok(key_hdr) = std::env::var("SUPABASE_SERVICE_ROLE_KEY").or_else(|_| std::env::var("SUPABASE_ANON_KEY")) {
                    debug!("parser: retrying supabase auth_url for envelope_id={} status={} ", envelope_id, resp.status().as_u16());
                    resp = client.get(auth_url).header("Authorization", format!("Bearer {}", key_hdr)).header("apikey", key_hdr.clone()).send().await?;
                }
            }
            if !resp.status().is_success() { error!("parser: fetch_bytes_failed envelope_id={} status={}", envelope_id, resp.status().as_u16()); return Err(format!("fetch_bytes_failed: {}", resp.status()).into()); }
            let b = resp.bytes().await?.to_vec();
println!("[metrics] record sms_parse_resolved_payload_bytes={} bytes", b.len());
            ::metrics::histogram!("sms_parse_resolved_payload_bytes").record(b.len() as f64);
            b
        } else {
            if let Some(path) = reader.resolve_payload_path(&payload_ref_s) {
                debug!("parser: reading payload from local CAS path for envelope_id={} path={} ", envelope_id, path.display());
let b = std::fs::read(path)?;
                println!("[metrics] record sms_parse_resolved_payload_bytes={} bytes", b.len());
                ::metrics::histogram!("sms_parse_resolved_payload_bytes").record(b.len() as f64);
                b
            } else { warn!("parser: could not resolve payload path for envelope_id={} payload_ref={}", envelope_id, payload_ref_s); counter!("sms_parse_resolve_payload_errors_total").increment(1); continue }
        };

        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        let reg_path = base.join("registry/sources").join(format!("{}.json", src_id));
        let spec = load_source_spec(&reg_path)?;
        let plan = spec.parse_plan_ref.clone().unwrap_or_else(|| "parse_plan:wix_calendar_v1".to_string());
        info!("parser: parsing envelope_id={} src_id={} plan={} bytes={}", envelope_id, src_id, plan, bytes.len());
        let parse_t0 = std::time::Instant::now();
        let recs: Vec<ParsedRecord> = match spec.parse_plan_ref.as_deref() {
            Some("parse_plan:wix_calendar_v1") | None => { let p = WixCalendarV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string()); p.parse(&bytes)? }
            Some("parse_plan:wix_warmup_v1") => { let p = WixWarmupV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string()); p.parse(&bytes)? }
            Some("parse_plan:darrells_html_v1") => { let p = DarrellsHtmlV1Parser::new(src_id.clone(), envelope_id.clone(), payload_ref_s.to_string()); p.parse(&bytes)? }
            Some(other) => { warn!("parser: skipping envelope {} with unsupported parse plan {}", envelope_id, other); Vec::new() }
        };
        let parse_secs = parse_t0.elapsed().as_secs_f64();
println!("[metrics] record sms_parse_duration_seconds={}s", parse_secs);
        ::metrics::histogram!("sms_parse_duration_seconds").record(parse_secs);
        if recs.is_empty() { warn!("parser: parser produced 0 records for envelope_id={} src_id={} plan={}", envelope_id, src_id, plan); counter!("sms_parse_empty_record_envelopes_total").increment(1); }
        for r in recs.clone() {
            let line = serde_json::to_string(&r)?;
            use std::io::Write; writeln!(out, "{}", line)?;
        }
        total_written += recs.len();
println!("[metrics] inc sms_parse_records_written_total by {}", recs.len());
        ::metrics::counter!("sms_parse_records_written_total").increment(recs.len() as u64);
    }

    Ok(ParseResultSummary {
        seen: total_seen,
        filtered_out: total_filtered,
        empty_record_envelopes: total_empty_records,
        written_records: total_written,
        output_file: prefixed_path.to_string_lossy().to_string(),
    })
}
