use crate::app::ports::NormalizeOutputPort;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use tracing::{info, warn};

/// File-based implementation of NormalizeOutputPort
/// Writes normalized entities to separate NDJSON files
pub struct FileNormalizeOutputAdapter {
    events_file: Mutex<std::io::BufWriter<std::fs::File>>,
    venues_file: Mutex<std::io::BufWriter<std::fs::File>>,
    artists_file: Mutex<std::io::BufWriter<std::fs::File>>,
    file_path_base: String,
}

impl FileNormalizeOutputAdapter {
    pub fn new(base_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let base = Path::new(base_path);
        let dir = base.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        
        // Generate file paths for each entity type
        let base_stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("normalized");
        let base_ext = base.extension().and_then(|s| s.to_str()).unwrap_or("ndjson");
        
        let events_path = dir.join(format!("{}_events.{}", base_stem, base_ext));
        let venues_path = dir.join(format!("{}_venues.{}", base_stem, base_ext));
        let artists_path = dir.join(format!("{}_artists.{}", base_stem, base_ext));
        
        info!("Creating normalized output files:");
        info!("  Events: {}", events_path.display());
        info!("  Venues: {}", venues_path.display());
        info!("  Artists: {}", artists_path.display());
        
        let events_file = std::io::BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&events_path)?
        );
        
        let venues_file = std::io::BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&venues_path)?
        );
        
        let artists_file = std::io::BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&artists_path)?
        );
        
        Ok(Self {
            events_file: Mutex::new(events_file),
            venues_file: Mutex::new(venues_file),
            artists_file: Mutex::new(artists_file),
            file_path_base: base_path.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl NormalizeOutputPort for FileNormalizeOutputAdapter {
    async fn write_normalized_record(&self, record: &crate::pipeline::processing::normalize::NormalizedRecord) -> anyhow::Result<()> {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        let line = serde_json::to_string(record)? + "\n";
        match &record.entity {
            NormalizedEntity::Event(_) => {
                if let Ok(mut f) = self.events_file.lock() {
                    f.write_all(line.as_bytes()).map_err(|e| anyhow::anyhow!("write events failed: {}", e))?;
                    f.flush().map_err(|e| anyhow::anyhow!("flush events failed: {}", e))?;
                } else {
                    warn!("normalize_output: failed to lock events file");
                }
            }
            NormalizedEntity::Venue(_) => {
                if let Ok(mut f) = self.venues_file.lock() {
                    f.write_all(line.as_bytes()).map_err(|e| anyhow::anyhow!("write venues failed: {}", e))?;
                    f.flush().map_err(|e| anyhow::anyhow!("flush venues failed: {}", e))?;
                } else {
                    warn!("normalize_output: failed to lock venues file");
                }
            }
            NormalizedEntity::Artist(_) => {
                if let Ok(mut f) = self.artists_file.lock() {
                    f.write_all(line.as_bytes()).map_err(|e| anyhow::anyhow!("write artists failed: {}", e))?;
                    f.flush().map_err(|e| anyhow::anyhow!("flush artists failed: {}", e))?;
                } else {
                    warn!("normalize_output: failed to lock artists file");
                }
            }
        }
        Ok(())
    }
}
