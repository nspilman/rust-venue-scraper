pub mod domain {
    // Core business types and rules. Keep this pure and free of I/O concerns.
    // As we migrate, copy pure types here first, then switch imports, then delete originals.
}

pub mod application {
    // Use-cases and ports (traits) that the domain needs to be executed.
    // Define traits for external concerns the use-cases depend on.
    use std::sync::Arc;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    pub struct ContentMeta {
        pub url: String,
        pub method: String,
        pub content_type: String,
        pub content_length: u64,
        pub sha256_hex: String,
    }

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    pub struct IngestResult {
        pub envelope_id: String,
        pub payload_ref: String,
        pub bytes_written: u64,
    }

    #[allow(dead_code)]
    #[async_trait::async_trait]
    pub trait StoragePort: Send + Sync {
        // Persist opaque payload bytes under a content-addressed key (e.g., sha256)
        async fn save_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String>;
        async fn load_raw(&self, key: &str) -> Result<Option<Vec<u8>>, String>;
        // Record an ingest event/envelope reference
        async fn record_ingest(&self, meta: &ContentMeta, payload_ref: &str) -> Result<String, String>;
    }

    #[allow(dead_code)]
    #[async_trait::async_trait]
    pub trait HttpClientPort: Send + Sync {
        async fn get(&self, url: &str) -> Result<(Vec<u8>, String /* content_type */), String>;
    }

    #[allow(dead_code)]
    pub trait ClockPort: Send + Sync {
        fn now_utc(&self) -> std::time::SystemTime;
    }

    #[allow(dead_code)]
    pub trait MetricsPort: Send + Sync {
        fn incr(&self, name: &str, value: u64);
        fn observe(&self, name: &str, value: f64);
    }

    #[allow(dead_code)]
    pub struct UseCases<P: StoragePort + ?Sized, H: HttpClientPort + ?Sized, M: MetricsPort + ?Sized, C: ClockPort + ?Sized> {
        pub storage: Arc<P>,
        pub http: Arc<H>,
        pub metrics: Arc<M>,
        pub clock: Arc<C>,
    }

    impl<P, H, M, C> UseCases<P, H, M, C>
    where
        P: StoragePort + ?Sized,
        H: HttpClientPort + ?Sized,
        M: MetricsPort + ?Sized,
        C: ClockPort + ?Sized,
    {
        #[allow(dead_code)]
        pub fn new(storage: Arc<P>, http: Arc<H>, metrics: Arc<M>, clock: Arc<C>) -> Self {
            Self { storage, http, metrics, clock }
        }

        // Skeleton use-case: ingest a single URL once and persist its payload
        #[allow(dead_code)]
        pub async fn ingest_source_once(&self, url: &str, method: &str) -> Result<IngestResult, String> {
            let t0 = std::time::Instant::now();
            self.metrics.incr("ingest_attempts", 1);

            let (bytes, content_type) = self.http.get(url).await?;
            let size = bytes.len() as u64;

            // Compute sha256 for content addressing
            let sha256_hex = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&bytes);
                hex::encode(h.finalize())
            };
            let payload_ref = format!("cas:sha256:{}", sha256_hex);

            // Save payload first (idempotent at storage layer if already present)
            self.storage.save_raw(&sha256_hex, bytes).await?;

            // Record envelope-like entry and get logical envelope_id back
            let meta = ContentMeta {
                url: url.to_string(),
                method: method.to_string(),
                content_type,
                content_length: size,
                sha256_hex: sha256_hex.clone(),
            };
            let envelope_id = self.storage.record_ingest(&meta, &payload_ref).await?;

            self.metrics.observe("ingest_duration_seconds", t0.elapsed().as_secs_f64());
            self.metrics.incr("ingest_bytes_total", size);

            Ok(IngestResult { envelope_id, payload_ref, bytes_written: size })
        }
    }
}

pub mod infrastructure {
    // Adapters that will implement application ports using concrete tech (reqwest, fs, db, etc.)
}

pub mod interface {
    // HTTP/CLI adapters that translate requests to application use-cases.
}

