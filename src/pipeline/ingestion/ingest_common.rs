use crate::pipeline::ingestion::envelope::{
    ChecksumMeta, EnvelopeSubmissionV1, LegalMeta, PayloadMeta, RequestMeta, TimingMeta,
};
use crate::common::error::{Result, ScraperError};
use crate::pipeline::ingestion::gateway::Gateway;
use crate::pipeline::ingestion::idempotency::compute_idempotency_key;
use crate::pipeline::ingestion::ingest_meta::IngestMeta;
use crate::pipeline::ingestion::rate_limiter::{Limits, RateLimiter};
use crate::pipeline::ingestion::registry::load_source_spec;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
use std::path::Path;
use std::time::Instant;
use tracing::debug;

/// Fetch payload bytes for a source defined in the registry and persist an ingest envelope via the gateway.
///
/// This centralizes the new ingestion behavior (registry lookup, cadence enforcement, rate limiting,
/// safety checks, idempotency, gateway accept, and cadence update) so individual ingestors can focus on parsing.
pub async fn fetch_payload_and_log(source_id: &str) -> Result<Vec<u8>> {
    // 1) Load registry entry
    let reg_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("registry/sources")
        .join(format!("{}.json", source_id));

    let spec = match load_source_spec(&reg_path) {
        Ok(spec) => {
            crate::observability::metrics::sources::registry_load_success();
            spec
        }
        Err(e) => {
            crate::observability::metrics::sources::registry_load_error();
            return Err(ScraperError::Api {
                message: format!("Failed to load registry for {}: {}", source_id, e),
            });
        }
    };

    if !spec.enabled {
        return Err(ScraperError::Api {
            message: format!("Source {} is disabled in registry", source_id),
        });
    }
    let ep = spec.endpoints.first().ok_or_else(|| ScraperError::Api {
        message: "No endpoint in registry".into(),
    })?;

    // 2) Cadence: enforce at most twice/day per source (unless bypassed)
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let bypass_cadence = std::env::var("SMS_BYPASS_CADENCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !bypass_cadence {
        let meta = IngestMeta::open_at_root(&data_root).map_err(|e| ScraperError::Api {
            message: format!("meta open failed: {}", e),
        })?;
        let now = chrono::Utc::now().timestamp();
        let min_interval_secs: i64 = 12 * 60 * 60;
        if let Some(last) =
            meta.get_last_fetched_at(&spec.source_id)
                .map_err(|e| ScraperError::Api {
                    message: format!("meta read failed: {}", e),
                })?
        {
            if now - last < min_interval_secs {
                // Cadence skipped
                return Err(ScraperError::Api {
                    message: "cadence_skip: fetched within last 12h".into(),
                });
            }
        }
        // Cadence allowed
    } else {
        // Cadence bypassed
    }

    // 3) Fetch bytes and headers with rate limiting per registry
    let rl = RateLimiter::new(Limits {
        requests_per_min: spec.rate_limits.requests_per_min,
        bytes_per_min: spec.rate_limits.bytes_per_min,
        concurrency: spec.rate_limits.concurrency.map(|c| c.max(1)),
    });
    // Build client - reqwest will automatically handle gzip/deflate decompression
    // when the "gzip" and "deflate" features are enabled
    let client = reqwest::Client::new();
    rl.acquire(0).await; // acquire for RPM/concurrency before send
    let fetch_t0 = Instant::now();
    let resp = client.get(&ep.url).send().await?;
    let status = resp.status().as_u16();
    let headers = resp.headers().clone();
    let bytes = resp.bytes().await?;
    let payload = bytes.to_vec();
    rl.acquire(payload.len() as u64).await; // account for bytes after size known

    // Record metrics
    let dur = fetch_t0.elapsed().as_secs_f64();
    if (200..=299).contains(&status) {
        crate::observability::metrics::sources::request_success();
        crate::observability::metrics::sources::request_duration(dur);
        crate::observability::metrics::sources::payload_bytes(payload.len());
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
        .unwrap_or(payload.len() as u64);
    let etag = headers
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let last_modified = headers
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 4) Safety checks against registry
    if content_length > spec.content.max_payload_size_bytes {
        return Err(ScraperError::Api {
            message: format!(
                "Payload too large: {} > {}",
                content_length, spec.content.max_payload_size_bytes
            ),
        });
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
        return Err(ScraperError::Api {
            message: format!(
                "MIME '{}' not in allow-list {:?}",
                content_type, spec.content.allowed_mime_types
            ),
        });
    }

    // 5) Compute checksum and idempotency key
    let sha_hex = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&payload);
        hex::encode(h.finalize())
    };
    let idk = compute_idempotency_key(
        &spec.source_id,
        &ep.url,
        etag.as_deref(),
        last_modified.as_deref(),
        &sha_hex,
    );

    // 6) Build envelope and accept via gateway (persist CAS + log)
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
            fetched_at: chrono::Utc::now(),
            gateway_received_at: None,
        },
        legal: LegalMeta {
            license_id: spec.policy.license_id.clone(),
        },
    };

    let gw = Gateway::new(data_root.clone());
    let accept_start = Instant::now();
    let stamped = gw.accept(env, &payload).map_err(|e| {
        crate::observability::metrics::gateway::cas_write_error();
        ScraperError::Api {
            message: format!("Gateway accept failed: {}", e),
        }
    })?;

    let accept_duration = accept_start.elapsed().as_secs_f64();

    // Record successful gateway and ingest log metrics
    crate::observability::metrics::gateway::envelope_accepted();
    crate::observability::metrics::gateway::processing_duration(accept_duration);
    crate::observability::metrics::gateway::cas_write_success();
    crate::observability::metrics::ingest_log::write_success();
    crate::observability::metrics::ingest_log::write_bytes(payload.len());

    debug!(
        "Accepted envelope {} with payload {}",
        stamped.envelope_id, stamped.payload_ref
    );

    // 7) Update cadence marker
    {
        let meta = IngestMeta::open_at_root(&data_root).map_err(|e| ScraperError::Api {
            message: format!("meta open failed: {}", e),
        })?;
        let now = chrono::Utc::now().timestamp();
        let _ = meta.set_last_fetched_at(&stamped.envelope.source_id, now);
    }

    Ok(payload)
}
