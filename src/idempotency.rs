use sha2::{Digest, Sha256};

pub fn compute_idempotency_key(
    source_id: &str,
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
    payload_sha256_hex: &str,
) -> String {
    // Simple canonical string; can be evolved later
    let mut s = String::new();
    s.push_str(source_id);
    s.push('|');
    s.push_str(url);
    s.push('|');
    if let Some(e) = etag { s.push_str(e); }
    s.push('|');
    if let Some(lm) = last_modified { s.push_str(lm); }
    s.push('|');
    s.push_str(payload_sha256_hex);

    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let out = hasher.finalize();
    hex::encode(out)
}
