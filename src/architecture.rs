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
    #[derive(Clone, Debug)]
    pub struct HttpGetResponse {
        pub bytes: Vec<u8>,
        pub content_type: String,
        pub content_length: u64,
        pub etag: Option<String>,
        pub last_modified: Option<String>,
        pub status: u16,
    }

    #[allow(dead_code)]
    #[async_trait::async_trait]
    pub trait HttpClientPort: Send + Sync {
        async fn get(&self, url: &str) -> Result<HttpGetResponse, String>;
    }

    #[allow(dead_code)]
    pub trait ClockPort: Send + Sync {
        fn now_utc(&self) -> std::time::SystemTime;
    }

    #[allow(dead_code)]
    pub trait MetricsPort: Send + Sync {
        fn incr(&self, name: &'static str, value: u64);
        fn observe(&self, name: &'static str, value: f64);
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

            let resp = self.http.get(url).await?;
            let size = resp.bytes.len() as u64;

            // Compute sha256 for content addressing
            let sha256_hex = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(&resp.bytes);
                hex::encode(h.finalize())
            };
            let payload_ref = format!("cas:sha256:{}", sha256_hex);

            // Save payload first (idempotent at storage layer if already present)
            self.storage.save_raw(&sha256_hex, resp.bytes).await?;

            // Record envelope-like entry and get logical envelope_id back
            let meta = ContentMeta {
                url: url.to_string(),
                method: method.to_string(),
                content_type: resp.content_type,
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
    use super::application::{ClockPort, ContentMeta, HttpClientPort, HttpGetResponse, MetricsPort, StoragePort};
    use serde_json::json;
    use std::path::{Path, PathBuf};

    pub struct ReqwestHttpClient;

    #[async_trait::async_trait]
    impl HttpClientPort for ReqwestHttpClient {
        async fn get(&self, url: &str) -> Result<HttpGetResponse, String> {
            use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, LAST_MODIFIED};
            let client = reqwest::Client::new();
            let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
            let status = resp.status().as_u16();
            let headers = resp.headers().clone();
            let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

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
            let etag = headers.get(ETAG).and_then(|v| v.to_str().ok()).map(|s| s.to_string());
            let last_modified = headers
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            Ok(HttpGetResponse { bytes, content_type, content_length, etag, last_modified, status })
        }
    }

    pub struct MetricsForwarder;
    impl MetricsPort for MetricsForwarder {
        fn incr(&self, name: &'static str, value: u64) {
            let c = ::metrics::counter!(name);
            c.increment(value);
        }
        fn observe(&self, name: &'static str, value: f64) {
            let h = ::metrics::histogram!(name);
            h.record(value);
        }
    }

    /// Simple clock adapter using std::time::SystemTime::now
    pub struct UtcClock;
    impl ClockPort for UtcClock {
        fn now_utc(&self) -> std::time::SystemTime {
            std::time::SystemTime::now()
        }
    }

    /// Storage adapter that persists to local CAS and appends to ingest log (NDJSON)
    pub struct LocalCasAndLog {
        root: PathBuf,
    }

    impl LocalCasAndLog {
        pub fn new<P: AsRef<Path>>(root: P) -> Self {
            Self { root: root.as_ref().to_path_buf() }
        }

        fn cas_path_for_key(&self, key: &str) -> PathBuf {
            // key is sha256 hex
            let (a, b) = (&key[0..2], &key[2..4]);
            self.root
                .join("cas")
                .join("sha256")
                .join(a)
                .join(b)
                .join(key)
        }

        fn ingest_log_path(&self) -> PathBuf {
            self.root.join("ingest_log").join("ingest.ndjson")
        }
    }

    #[async_trait::async_trait]
    impl StoragePort for LocalCasAndLog {
        async fn save_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
            let path = self.cas_path_for_key(key);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("save_raw:create_dir_all parent={} err={}", parent.display(), e))?;
            }
            // Write atomically by writing to tmp then rename
            let tmp = path.with_extension("tmp");
            if tokio::fs::try_exists(&path)
                .await
                .map_err(|e| format!("save_raw:try_exists path={} err={}", path.display(), e))?
            {
                return Ok(()); // already present
            }
            tokio::fs::write(&tmp, &bytes)
                .await
                .map_err(|e| format!("save_raw:write tmp={} err={}", tmp.display(), e))?;
            tokio::fs::rename(&tmp, &path)
                .await
                .map_err(|e| format!("save_raw:rename tmp={} -> path={} err={}", tmp.display(), path.display(), e))?;
            Ok(())
        }

        async fn load_raw(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            let path = self.cas_path_for_key(key);
            if !tokio::fs::try_exists(&path)
                .await
                .map_err(|e| format!("load_raw:try_exists path={} err={}", path.display(), e))?
            {
                return Ok(None);
            }
            let b = tokio::fs::read(&path)
                .await
                .map_err(|e| format!("load_raw:read path={} err={}", path.display(), e))?;
            Ok(Some(b))
        }

        async fn record_ingest(&self, meta: &ContentMeta, payload_ref: &str) -> Result<String, String> {
            let logp = self.ingest_log_path();
            if let Some(parent) = logp.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("record_ingest:create_dir_all parent={} err={}", parent.display(), e))?;
            }
            let env_id = uuid::Uuid::new_v4().to_string();
            let obj = json!({
                "envelope_id": env_id,
                "payload_ref": payload_ref,
                "meta": {
                    "url": meta.url,
                    "method": meta.method,
                    "content_type": meta.content_type,
                    "content_length": meta.content_length,
                    "checksum": { "sha256": meta.sha256_hex },
                }
            });
            let line = serde_json::to_string(&obj)
                .map_err(|e| format!("record_ingest:serde_json_to_string err={}", e))? + "\n";
            use tokio::io::AsyncWriteExt;
            let mut f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&logp)
                .await
                .map_err(|e| format!("record_ingest:open path={} err={}", logp.display(), e))?;
            f.write_all(line.as_bytes())
                .await
                .map_err(|e| format!("record_ingest:write_all path={} err={}", logp.display(), e))?;
            Ok(obj["envelope_id"].as_str().unwrap_or("").to_string())
        }
    }
}

pub mod interface {
    // HTTP/CLI adapters that translate requests to application use-cases.
}

