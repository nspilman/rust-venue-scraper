use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn write_cas(root: &Path, bytes: &[u8]) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    let dir = root.join("sha256").join(&hex[0..2]).join(&hex[2..4]);
    fs::create_dir_all(&dir)?;
    let path = dir.join(&hex);
    if !path.exists() {
        fs::write(&path, bytes)?;
    }
    Ok(format!("cas:sha256:{}", hex))
}
