use crate::app::ports::{CadencePort, GatewayPort, HttpClientPort, RateLimiterPort};
use crate::pipeline::ingestion::envelope::{ChecksumMeta, EnvelopeSubmissionV1, LegalMeta, PayloadMeta, RequestMeta, TimingMeta};

pub struct IngestUseCase<R: RateLimiterPort + ?Sized, C: CadencePort + ?Sized, H: HttpClientPort + ?Sized, G: GatewayPort + ?Sized> {
    pub rate: Box<R>,
    pub cadence: Box<C>,
    pub http: Box<H>,
    pub gateway: Box<G>,
}

impl<R: RateLimiterPort + ?Sized, C: CadencePort + ?Sized, H: HttpClientPort + ?Sized, G: GatewayPort + ?Sized> IngestUseCase<R, C, H, G> {
    pub fn new(rate: Box<R>, cadence: Box<C>, http: Box<H>, gateway: Box<G>) -> Self {
        Self { rate, cadence, http, gateway }
    }

    pub async fn ingest_once(&self, source_id: &str, url: &str, method: &str, max_payload_bytes: u64, allowed_mime: &[String], license_id: &str)
        -> Result<(String, String, usize), String> {
        // cadence
        if !self.cadence.should_run(source_id, 12 * 60 * 60).await? {
            return Err("cadence_skip".into());
        }
        // rate limit and fetch
        self.rate.acquire(0).await;
        let resp = self.http.get(url).await?;
        self.rate.acquire(resp.content_length).await;
        // safety
        if resp.content_length > max_payload_bytes { return Err("payload_too_large".into()); }
        let base = resp.content_type.split(';').next().unwrap_or("").trim().to_string();
        if !allowed_mime.iter().any(|m| m == &base) { return Err("mime_not_allowed".into()); }
        // checksum
        let sha_hex = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&resp.bytes);
            hex::encode(h.finalize())
        };
        // build envelope
        let env = EnvelopeSubmissionV1 {
            envelope_version: "1.0.0".to_string(),
            source_id: source_id.to_string(),
            idempotency_key: format!("{}:{}:{}:{}", source_id, url, resp.etag.clone().unwrap_or_default(), sha_hex),
            payload_meta: PayloadMeta { mime_type: resp.content_type.clone(), size_bytes: resp.content_length, checksum: ChecksumMeta { sha256: sha_hex.clone() } },
            request: RequestMeta { url: url.to_string(), method: method.to_string(), status: Some(resp.status), etag: resp.etag.clone(), last_modified: resp.last_modified.clone() },
            timing: TimingMeta { fetched_at: chrono::Utc::now(), gateway_received_at: None },
            legal: LegalMeta { license_id: license_id.to_string() },
        };
        let stamped = self.gateway.accept(env, resp.bytes).await?;
        self.cadence.mark_run(source_id).await?;
        Ok((stamped.envelope_id, stamped.payload_ref, stamped.envelope.payload_meta.size_bytes as usize))
    }
}

