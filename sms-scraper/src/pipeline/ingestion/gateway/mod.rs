pub mod cas_fs;
pub mod cas_supabase;
pub mod ingest_log;

use crate::pipeline::ingestion::envelope::{EnvelopeSubmissionV1, StampedEnvelopeV1};
use crate::pipeline::ingestion::ingest_meta::IngestMeta;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub struct Gateway {
    root: PathBuf,
}

impl Gateway {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        let root = root.into();
        let cas_dir = root.join("cas");
        let log_dir = root.join("ingest_log");
        let _ = fs::create_dir_all(&cas_dir);
        let _ = fs::create_dir_all(&log_dir);
        Self { root }
    }

    // Dedupe index now stored in SQLite (ingest_log/meta.db) via IngestMeta

    pub fn accept(
        &self,
        env: EnvelopeSubmissionV1,
        payload_bytes: &[u8],
    ) -> anyhow::Result<StampedEnvelopeV1> {
        let t0 = std::time::Instant::now();

        // Check if cadence is bypassed
        let bypass_cadence = std::env::var("SMS_BYPASS_CADENCE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        // Dedupe by idempotency_key (SQLite-backed) - only if not bypassing cadence
        let meta = IngestMeta::open_at_root(&self.root)?;
        let idk = env.idempotency_key.clone();
        if !bypass_cadence {
            if let Some(existing_id) = meta.get_envelope_by_idk(&idk)? {
                crate::observability::metrics::gateway::envelope_deduplicated();
                let accepted_at = Utc::now();
                let envelope_id = Uuid::new_v4().to_string();
                let dup = StampedEnvelopeV1 {
                    envelope_version: env.envelope_version.clone(),
                    envelope_id: envelope_id.clone(),
                    accepted_at,
                    payload_ref: String::new(),
                    dedupe_of: Some(existing_id.clone()),
                    envelope: EnvelopeSubmissionV1 {
                        timing: crate::pipeline::ingestion::envelope::TimingMeta {
                            gateway_received_at: Some(accepted_at),
                            ..env.timing.clone()
                        },
                        ..env.clone()
                    },
                };
                ingest_log::append_rotating(&self.root.join("ingest_log"), &dup)?;
                let dur = t0.elapsed().as_secs_f64();
                crate::observability::metrics::gateway::processing_duration(dur);
                return Ok(dup);
            }
        }

        let _bytes = payload_bytes.len();
        crate::observability::metrics::gateway::envelope_accepted();
        let accepted_at = Utc::now();
        let envelope_id = Uuid::new_v4().to_string();

        // Write payload to CAS (Supabase if configured, otherwise local FS)
        let _cas_t0 = std::time::Instant::now();
        let payload_ref = if (std::env::var("SUPABASE_URL").is_ok()
            || std::env::var("SUPABASE_PROJECT_REF").is_ok())
            && std::env::var("SUPABASE_SERVICE_ROLE_KEY").is_ok()
            && std::env::var("SUPABASE_BUCKET").is_ok()
        {
            let result = cas_supabase::write_cas_supabase(payload_bytes);
            match &result {
                Ok(_) => crate::observability::metrics::gateway::cas_write_success(),
                Err(_) => crate::observability::metrics::gateway::cas_write_error(),
            }
            result?
        } else {
            let result = cas_fs::write_cas(&self.root.join("cas"), payload_bytes);
            match &result {
                Ok(_) => crate::observability::metrics::gateway::cas_write_success(),
                Err(_) => crate::observability::metrics::gateway::cas_write_error(),
            }
            result?
        };

        let stamped = StampedEnvelopeV1 {
            envelope_version: env.envelope_version.clone(),
            envelope_id: envelope_id.clone(),
            accepted_at,
            payload_ref: payload_ref.clone(),
            dedupe_of: None,
            envelope: EnvelopeSubmissionV1 {
                timing: crate::pipeline::ingestion::envelope::TimingMeta {
                    gateway_received_at: Some(accepted_at),
                    ..env.timing.clone()
                },
                ..env.clone()
            },
        };

        // First time: append log and index
        ingest_log::append_rotating(&self.root.join("ingest_log"), &stamped)?;
        meta.put_dedupe_mapping(&idk, &envelope_id)?;

        let dur = t0.elapsed().as_secs_f64();
        crate::observability::metrics::gateway::processing_duration(dur);
        Ok(stamped)
    }
}
