use crate::common::constants;
use crate::pipeline::ingestion::envelope::{
    ChecksumMeta, EnvelopeSubmissionV1, LegalMeta, PayloadMeta, RequestMeta, TimingMeta,
};
use crate::pipeline::ingestion::gateway::Gateway;
use crate::pipeline::ingestion::idempotency::compute_idempotency_key;
use crate::pipeline::ingestion::ingest_meta::IngestMeta;
use crate::pipeline::ingestion::rate_limiter::{Limits, RateLimiter};
use crate::pipeline::ingestion::registry::load_source_spec;
use crate::pipeline::storage::Storage;
use chrono::Utc;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};

fn data_root_path_from_arg(data_root: &str) -> PathBuf {
    let p = PathBuf::from(data_root);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
    }
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

pub async fn gateway_once(
    _storage: Arc<dyn Storage>,
    params: GatewayOnceParams,
) -> Result<GatewayOnceResult, Box<dyn std::error::Error>> {
    let source = params
        .source_id
        .unwrap_or_else(|| constants::BLUE_MOON_API.to_string());
    if params.bypass_cadence.unwrap_or(false) {
        std::env::set_var("SMS_BYPASS_CADENCE", "1");
    }
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let reg_path = base
        .join("registry/sources")
        .join(format!("{}.json", source));
    let spec = match load_source_spec(&reg_path) {
        Ok(spec) => {
            crate::observability::metrics::sources::registry_load_success();
            spec
        }
        Err(e) => {
            crate::observability::metrics::sources::registry_load_error();
            return Err(format!("Failed to load registry: {e}").into());
        }
    };
    if !spec.enabled {
        return Err("Source is disabled".into());
    }
    let ep = spec.endpoints.first().ok_or("No endpoint in registry")?;

    // Cadence check
    let data_root = data_root_path_from_arg(params.data_root.as_deref().unwrap_or("data"));
    let bypass = std::env::var("SMS_BYPASS_CADENCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !bypass {
        let meta = IngestMeta::open_at_root(&data_root)?;
        let now = Utc::now().timestamp();
        let min_interval_secs: i64 = 12 * 60 * 60;
        if let Some(last) = meta.get_last_fetched_at(&spec.source_id)? {
            if now - last < min_interval_secs {
                return Err("cadence_skip: fetched within last 12h".into());
            }
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
    if (200..=299).contains(&status) {
        crate::observability::metrics::sources::request_success();
        crate::observability::metrics::sources::request_duration(dur);
        crate::observability::metrics::sources::payload_bytes(bytes.len());
    } else {
        crate::observability::metrics::sources::request_error();
    }

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let content_length: u64 = headers
        .get(CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(bytes.len() as u64);
    let etag = headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = headers
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if content_length > spec.content.max_payload_size_bytes {
        return Err(format!(
            "Payload too large: {} > {}",
            content_length, spec.content.max_payload_size_bytes
        )
        .into());
    }
    let content_type_base = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if !spec
        .content
        .allowed_mime_types
        .iter()
        .any(|m| m == &content_type_base)
    {
        return Err(format!(
            "MIME '{}' not in allow-list {:?}",
            content_type, spec.content.allowed_mime_types
        )
        .into());
    }

    let sha_hex = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&bytes);
        hex::encode(h.finalize())
    };
    let idk = compute_idempotency_key(
        &spec.source_id,
        &ep.url,
        etag.as_deref(),
        last_modified.as_deref(),
        &sha_hex,
    );

    let env = EnvelopeSubmissionV1 {
        envelope_version: "1.0.0".to_string(),
        source_id: spec.source_id.clone(),
        idempotency_key: idk,
        payload_meta: PayloadMeta {
            mime_type: content_type,
            size_bytes: content_length,
            checksum: ChecksumMeta { sha256: sha_hex },
        },
        request: RequestMeta {
            url: ep.url.clone(),
            method: ep.method.clone(),
            status: Some(status),
            etag,
            last_modified,
        },
        timing: TimingMeta {
            fetched_at: Utc::now(),
            gateway_received_at: None,
        },
        legal: LegalMeta {
            license_id: spec.policy.license_id.clone(),
        },
    };

    let gw = Gateway::new(data_root.clone());
    let stamped = gw.accept(env, &bytes)?;
    let _ = IngestMeta::open_at_root(&data_root)?
        .set_last_fetched_at(&stamped.envelope.source_id, Utc::now().timestamp());

    Ok(GatewayOnceResult {
        source_id: spec.source_id,
        envelope_id: stamped.envelope_id,
        payload_bytes: bytes.len(),
        ingest_log: data_root
            .join("ingest_log/ingest.ndjson")
            .to_string_lossy()
            .to_string(),
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
    pub normalize: Option<bool>,
    pub quality_gate: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ParseResultSummary {
    pub seen: usize,
    pub filtered_out: usize,
    pub empty_record_envelopes: usize,
    pub written_records: usize,
    pub output_file: String,
}

pub async fn parse_run(
    _storage: Arc<dyn Storage>,
    params: ParseParams,
) -> Result<ParseResultSummary, Box<dyn std::error::Error>> {
    use crate::pipeline::ingestion::ingest_log_reader::IngestLogReader;
    use crate::app::parse_use_case::ParseUseCase;
    use crate::infra::{payload_store::CasPayloadStore, registry_adapter::JsonRegistry, parser_factory::DefaultParserFactory};

    let consumer = params.consumer.unwrap_or_else(|| "parser".to_string());
    let max = params.max.unwrap_or(50);
    let data_root_s = params.data_root.unwrap_or_else(|| "data".to_string());
    let output = params.output.unwrap_or_else(|| "parsed.ndjson".to_string());

    // Validation: quality gate requires normalization to be enabled
    if params.quality_gate.unwrap_or(false) && !params.normalize.unwrap_or(false) {
        return Err("Quality gate requires normalization to be enabled. Use both --normalize and --quality-gate flags.".into());
    }

    let reader = IngestLogReader::new(data_root_path_from_arg(&data_root_s));
    let (lines, _last) = reader.read_next(&consumer, max)?;
    info!("parser: read {} log lines from ingest log", lines.len());
    crate::observability::metrics::parser::batch_size(lines.len());
    if lines.is_empty() {
        return Ok(ParseResultSummary { seen: 0, filtered_out: 0, empty_record_envelopes: 0, written_records: 0, output_file: "".to_string() });
    }

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let base_out = Path::new(&output);
    let output_dir = Path::new("output");
    std::fs::create_dir_all(output_dir)?;
    let file = base_out.file_name().unwrap_or_else(|| std::ffi::OsStr::new("parsed.ndjson"));
    let prefixed_path = output_dir.join(format!("{}_{}", ts, file.to_string_lossy()));
    let mut out = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&prefixed_path)?;

    // Wire ports and use-cases
    let parse_uc = ParseUseCase::new(Box::new(JsonRegistry), Box::new(CasPayloadStore), Box::new(DefaultParserFactory));
    
    // Optionally create normalize use case if normalization is enabled
    let normalize_uc = if params.normalize.unwrap_or(false) {
        use crate::app::normalize_use_case::NormalizeUseCase;
        use crate::pipeline::processing::normalize::DefaultNormalizer;
        use crate::infra::normalize_output_adapter::FileNormalizeOutputAdapter;
        
        let normalized_output = output.replace(".ndjson", "_normalized.ndjson");
        let normalized_path = output_dir.join(format!("{}_{}", ts, Path::new(&normalized_output).file_name().unwrap_or_else(|| std::ffi::OsStr::new("normalized.ndjson")).to_string_lossy()));
        let normalize_adapter = FileNormalizeOutputAdapter::new(&normalized_path.to_string_lossy())?;
        
        Some((NormalizeUseCase::new(Box::new(DefaultNormalizer { geocoder: None }), Box::new(normalize_adapter)), normalized_path))
    } else {
        None
    };
    
    // Optionally create quality gate use case if quality gate is enabled (requires normalization)
    let quality_gate_uc = if params.quality_gate.unwrap_or(false) && params.normalize.unwrap_or(false) {
        use crate::app::quality_gate_use_case::QualityGateUseCase;
        use crate::infra::quality_gate_output_adapter::FileQualityGateOutputAdapter;
        
        let accepted_output = output.replace(".ndjson", "_accepted.ndjson");
        let quarantined_output = output.replace(".ndjson", "_quarantined.ndjson");
        
        let accepted_path = output_dir.join(format!("{}_{}", ts, Path::new(&accepted_output).file_name().unwrap_or_else(|| std::ffi::OsStr::new("accepted.ndjson")).to_string_lossy()));
        let quarantined_path = output_dir.join(format!("{}_{}", ts, Path::new(&quarantined_output).file_name().unwrap_or_else(|| std::ffi::OsStr::new("quarantined.ndjson")).to_string_lossy()));
        
        let accepted_adapter = FileQualityGateOutputAdapter::new(&accepted_path.to_string_lossy())?;
        let quarantined_adapter = FileQualityGateOutputAdapter::new(&quarantined_path.to_string_lossy())?;
        
        Some((QualityGateUseCase::with_default_quality_gate(Box::new(accepted_adapter), Box::new(quarantined_adapter)), accepted_path, quarantined_path))
    } else {
        None
    };
    
    use crate::app::ports::RegistryPort;

    let mut total_seen = 0usize;
    let mut total_filtered = 0usize;
    let mut total_written = 0usize;
    let mut total_empty_records = 0usize;

    for line in lines {
        total_seen += 1;
        let val: serde_json::Value = match serde_json::from_str(&line) { Ok(v) => v, Err(e) => { warn!("parser: skipping invalid JSON line: {}", e); continue; } };
        let mut payload_ref_s = val.get("payload_ref").and_then(|v| v.as_str()).or_else(|| val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str())).unwrap_or("").to_string();
        let envelope_id = val.get("envelope_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let src_id = val.get("envelope").and_then(|e| e.get("source_id")).and_then(|v| v.as_str()).unwrap_or("").to_string();

        if payload_ref_s.is_empty() {
            if let Some(dedupe_of) = val.get("dedupe_of").and_then(|v| v.as_str()) {
                if let Ok(Some(orig_line)) = reader.find_envelope_by_id(dedupe_of) {
                    if let Ok(orig_val) = serde_json::from_str::<serde_json::Value>(&orig_line) {
                        if let Some(pr) = orig_val.get("payload_ref").and_then(|v| v.as_str()).or_else(|| orig_val.get("envelope").and_then(|e| e.get("payload_ref")).and_then(|v| v.as_str())) {
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
        if payload_ref_s.is_empty() {
            if let Some(sha) = val.get("envelope").and_then(|e| e.get("payload_meta")).and_then(|pm| pm.get("checksum")).and_then(|c| c.get("sha256")).and_then(|s| s.as_str()) {
                payload_ref_s = format!("cas:sha256:{}", sha);
                info!("parser: synthesized payload_ref from checksum for envelope_id={}", envelope_id);
            }
        }
        if let Some(filter) = &params.source_id { if src_id != *filter { total_filtered += 1; continue; } }
        if payload_ref_s.is_empty() || src_id.is_empty() { warn!("parser: skipping envelope with missing fields: envelope_id='{}' src_id='{}' payload_ref_present={}", envelope_id, src_id, !payload_ref_s.is_empty()); continue; }

        // Use use-case to resolve and parse
        let reg = crate::infra::registry_adapter::JsonRegistry;
        let plan = reg.load_parse_plan(&src_id).await.unwrap_or_else(|_| "parse_plan:wix_calendar_v1".to_string());
        info!("parser: parsing envelope_id={} src_id={} plan={} payload_ref={} ", envelope_id, src_id, plan, payload_ref_s);
        let rec_lines = match parse_uc.parse_one(&src_id, &envelope_id, &payload_ref_s).await {
            Ok(lines) => {
                crate::observability::metrics::parser::parse_success();
                lines
            },
            Err(e) => { 
                warn!("parser: parse_failed envelope_id={} err={}", envelope_id, e); 
                crate::observability::metrics::parser::parse_error();
                total_empty_records += 1; 
                Vec::new() 
            }
        };
        if rec_lines.is_empty() { total_empty_records += 1; }
        
        // Write parsed records to output
        for line in rec_lines.iter() { use std::io::Write; writeln!(out, "{}", line)?; }
        total_written += rec_lines.len();
        
        // Storage for normalized records to pass to quality gate
        let mut normalized_records: Vec<crate::pipeline::processing::normalize::NormalizedRecord> = Vec::new();
        
        // Optionally normalize parsed records
        if let Some((ref norm_uc, _)) = normalize_uc {
            if !rec_lines.is_empty() {
                // Convert JSON lines to ParsedRecords for normalization
                let parsed_records: Vec<crate::pipeline::processing::parser::ParsedRecord> = rec_lines
                    .iter()
                    .filter_map(|line| {
                        match serde_json::from_str::<crate::pipeline::processing::parser::ParsedRecord>(line) {
                            Ok(record) => Some(record),
                            Err(e) => {
                                warn!("normalize: failed to deserialize parsed record: {}", e);
                                None
                            }
                        }
                    })
                    .collect();
                
                if !parsed_records.is_empty() {
                    info!("normalize: processing {} parsed records from envelope_id={}", parsed_records.len(), envelope_id);
                    match norm_uc.normalize_batch(&parsed_records).await {
                        Ok(batch_normalized_records) => {
                            info!("normalize: successfully normalized batch from envelope_id={}", envelope_id);
                            normalized_records.extend(batch_normalized_records);
                        },
                        Err(e) => {
                            warn!("normalize: failed to normalize batch from envelope_id={}: {}", envelope_id, e);
                        }
                    }
                }
            }
        }
        
        // Optionally run quality gate assessment on normalized records
        if let Some((ref qg_uc, _, _)) = quality_gate_uc {
            if !normalized_records.is_empty() {
                info!("quality_gate: processing {} normalized records from envelope_id={}", normalized_records.len(), envelope_id);
                match qg_uc.assess_batch(&normalized_records).await {
                    Ok(assessed_records) => {
                        info!("quality_gate: successfully assessed {} records from envelope_id={}", assessed_records.len(), envelope_id);
                    },
                    Err(e) => {
                        warn!("quality_gate: failed to assess batch from envelope_id={}: {}", envelope_id, e);
                    }
                }
            }
        }
        
        // Record gateway metrics for records ingested from this envelope
        if !rec_lines.is_empty() {
            crate::observability::metrics::gateway::records_ingested(rec_lines.len() as u64);
        }
    }

    // Record final parsing metrics
    crate::observability::metrics::parser::records_extracted(total_written as u64);

    Ok(ParseResultSummary { seen: total_seen, filtered_out: total_filtered, empty_record_envelopes: total_empty_records, written_records: total_written, output_file: prefixed_path.to_string_lossy().to_string() })
}
