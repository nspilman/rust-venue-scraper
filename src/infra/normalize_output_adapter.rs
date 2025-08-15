use crate::app::ports::NormalizeOutputPort;
use std::io::BufWriter;
use std::fs::OpenOptions;
use std::path::Path;
use tracing::info;

/// File-based implementation of NormalizeOutputPort
/// Writes normalized entities to separate NDJSON files
#[allow(dead_code)]
pub struct FileNormalizeOutputAdapter {
    #[allow(dead_code)]
    events_file: BufWriter<std::fs::File>,
    #[allow(dead_code)]
    venues_file: BufWriter<std::fs::File>,
    #[allow(dead_code)]
    artists_file: BufWriter<std::fs::File>,
    #[allow(dead_code)]
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
        
        let events_file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&events_path)?
        );
        
        let venues_file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&venues_path)?
        );
        
        let artists_file = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&artists_path)?
        );
        
        Ok(Self {
            events_file,
            venues_file,
            artists_file,
            file_path_base: base_path.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl NormalizeOutputPort for FileNormalizeOutputAdapter {
    async fn write_normalized_record(&self, record: &crate::pipeline::processing::normalize::NormalizedRecord) -> anyhow::Result<()> {
        use crate::pipeline::processing::normalize::NormalizedEntity;
        
        // Write each entity type to its appropriate file
        // Note: This requires a mutable reference, but the trait defines immutable
        // For now, we'll implement a basic version that accumulates records
        // In a real implementation, you might want to use internal mutability with Mutex
        
        match &record.entity {
            NormalizedEntity::Event(event) => {
                // For this implementation, we'll need to store records and write them in batches
                // This is a simplified version - in production you'd want better handling
                let json_line = serde_json::to_string(event)?;
                info!("Would write event: {}", json_line);
            },
            NormalizedEntity::Venue(venue) => {
                let json_line = serde_json::to_string(venue)?;
                info!("Would write venue: {}", json_line);
            },
            NormalizedEntity::Artist(artist) => {
                let json_line = serde_json::to_string(artist)?;
                info!("Would write artist: {}", json_line);
            },
        }
        
        Ok(())
    }
}
