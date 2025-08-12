use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use crate::ingest_meta::IngestMeta;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ConsumerOffset {
    pub file: String,
    pub byte_offset: u64,
    pub envelope_id: Option<String>,
}

pub struct IngestLogReader {
    root: PathBuf,
}

impl IngestLogReader {
    pub fn new<P: Into<PathBuf>>(data_root: P) -> Self {
        let root = data_root.into();
        Self { root }
    }

    fn log_path(&self) -> PathBuf {
        self.root.join("ingest_log").join("ingest.ndjson")
    }

    fn offsets_dir(&self) -> PathBuf { self.root.join("ingest_log").join("offsets") }

    fn offset_path(&self, consumer: &str) -> PathBuf {
        self.offsets_dir().join(format!("{}.json", consumer))
    }

    fn load_offset(&self, consumer: &str) -> ConsumerOffset {
        // Read from SQLite meta
        if let Ok(meta) = IngestMeta::open_at_root(&self.root) {
            if let Ok((byte_offset, envelope_id)) = meta.get_offset(consumer) {
                return ConsumerOffset { file: "ingest.ndjson".to_string(), byte_offset, envelope_id };
            }
        }
        ConsumerOffset { file: "ingest.ndjson".to_string(), byte_offset: 0, envelope_id: None }
    }

    fn save_offset(&self, consumer: &str, off: &ConsumerOffset) -> std::io::Result<()> {
        // Write to SQLite meta
        if let Ok(meta) = IngestMeta::open_at_root(&self.root) {
            let _ = meta.set_offset(consumer, off.byte_offset, off.envelope_id.as_deref());
        }
        Ok(())
    }

    pub fn status(&self, consumer: &str) -> std::io::Result<(ConsumerOffset, u64, u64)> {
        let mut off = self.load_offset(consumer);
        let log_path = self.log_path();
        let end = fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
        if off.byte_offset > end { off.byte_offset = 0; }
        let lag = end.saturating_sub(off.byte_offset);
        Ok((off, end, lag))
    }

    pub fn read_next(&self, consumer: &str, max: usize) -> std::io::Result<(Vec<String>, Option<String>)> {
        let mut off = self.load_offset(consumer);
        let path = self.log_path();
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        // Handle rotation: if stored offset is beyond current file end, reset to 0
        let end = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if off.byte_offset > end { off.byte_offset = 0; }
        reader.seek(SeekFrom::Start(off.byte_offset))?;

        let mut lines = Vec::new();
        let mut last_env: Option<String> = None;

        for _ in 0..max {
            let mut buf = String::new();
            let bytes = reader.read_line(&mut buf)?;
            if bytes == 0 { break; } // EOF
            // Trim newline for cleaner output but we keep byte math using bytes
            if let Some(stripped) = buf.strip_suffix('\n') { buf = stripped.to_string(); }
            if buf.trim().is_empty() { continue; }
            // Capture envelope_id for ack convenience
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&buf) {
                if let Some(id) = val.get("envelope_id").and_then(|v| v.as_str()) {
                    last_env = Some(id.to_string());
                }
            }
            lines.push(buf);
        }

        Ok((lines, last_env))
    }

    pub fn ack_through(&self, consumer: &str, envelope_id: &str) -> std::io::Result<ConsumerOffset> {
        // Advance from current offset up to and including the line with envelope_id
        let mut off = self.load_offset(consumer);
        let path = self.log_path();
        let mut file = File::open(&path)?;
        file.seek(SeekFrom::Start(off.byte_offset))?;
        let mut reader = BufReader::new(file);

        let mut cur = off.byte_offset;
        let mut buf = String::new();
        let mut found = false;
        loop {
            buf.clear();
            let read = reader.read_line(&mut buf)?;
            if read == 0 { break; } // EOF
            cur += read as u64;
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&buf) {
                if val.get("envelope_id").and_then(|v| v.as_str()) == Some(envelope_id) {
                    found = true;
                    break;
                }
            }
        }
        if !found {
            // No movement if we didn't find the id from current offset
            return Ok(off);
        }
        off.byte_offset = cur;
        off.envelope_id = Some(envelope_id.to_string());
        self.save_offset(consumer, &off)?;
        Ok(off)
    }

    pub fn resolve_payload_path(&self, payload_ref: &str) -> Option<PathBuf> {
        // Expect payload_ref like "cas:sha256:<hex>"
        let prefix = "cas:sha256:";
        if !payload_ref.starts_with(prefix) { return None; }
        let hex = &payload_ref[prefix.len()..];
        if hex.len() < 4 { return None; }
        let p = self.root
            .join("cas").join("sha256")
            .join(&hex[0..2]).join(&hex[2..4])
            .join(hex);
        Some(p)
    }

    pub fn find_envelope_by_id(&self, envelope_id: &str) -> std::io::Result<Option<String>> {
        // Linear scan of active log file (sufficient for now)
        let path = self.log_path();
        if !path.exists() { return Ok(None); }
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let l = line?;
            if l.contains(envelope_id) {
                // Quick filter; confirm
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&l) {
                    if val.get("envelope_id").and_then(|v| v.as_str()) == Some(envelope_id) {
                        return Ok(Some(l));
                    }
                }
            }
        }
        Ok(None)
    }
}
