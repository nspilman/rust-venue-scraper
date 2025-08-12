use sha2::{Digest, Sha256};

/// Uploads bytes to Supabase Storage in a content-addressed path and returns payload_ref "cas:sha256:<hex>".
/// Config via env:
/// - SUPABASE_URL (e.g., https://xyzcompany.supabase.co) OR SUPABASE_PROJECT_REF (e.g., ihkgojiseqpwinwdowvm)
/// - SUPABASE_SERVICE_ROLE_KEY (service role key)
/// - SUPABASE_BUCKET (bucket name)
/// - SUPABASE_PREFIX (optional path prefix inside bucket)
pub fn write_cas_supabase(bytes: &[u8]) -> anyhow::Result<String> {
    // Allow either a full URL or a project ref
    let url = match std::env::var("SUPABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            let project_ref = std::env::var("SUPABASE_PROJECT_REF")?;
            format!("https://{}.supabase.co", project_ref)
        }
    };

    let key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")?;
    let bucket = std::env::var("SUPABASE_BUCKET")?;
    let prefix = std::env::var("SUPABASE_PREFIX").unwrap_or_else(|_| String::new());

    // compute hash and object path
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hex = hex::encode(hasher.finalize());
    let path = if prefix.is_empty() {
        format!("sha256/{}/{}/{}", &hex[0..2], &hex[2..4], &hex)
    } else {
        format!("{}/sha256/{}/{}/{}", prefix.trim_end_matches('/'), &hex[0..2], &hex[2..4], &hex)
    };

    // Upload with upsert=true (idempotent for same content)
    let endpoint = format!("{}/storage/v1/object/{}/{}", url.trim_end_matches('/'), bucket, path);

    // Execute the HTTP call using the async client within a safe blocking section of the Tokio runtime
    let result = tokio::task::block_in_place(|| {
        let fut = async move {
            let client = reqwest::Client::new();
            let resp = client
                .put(&endpoint)
                .header("Authorization", format!("Bearer {}", key))
                .header("apikey", key.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .query(&[("upsert", "true")])
                .body(bytes.to_vec())
                .send()
                .await?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Ok::<_, reqwest::Error>((status, body))
        };
        tokio::runtime::Handle::current().block_on(fut)
    })?;

    let (status, body) = result;
    if !status.is_success() {
        return Err(anyhow::anyhow!("Supabase upload failed: {} - {}", status, body));
    }

    Ok(format!("cas:sha256:{}", hex))
}
