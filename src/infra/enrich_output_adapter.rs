use crate::app::ports::EnrichOutputPort;
use crate::pipeline::processing::enrich::EnrichedRecord;
use std::io::{BufWriter, Write};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

/// File-based implementation of EnrichOutputPort
/// Writes enriched records to NDJSON files
pub struct FileEnrichOutputAdapter {
    file_writer: Mutex<BufWriter<std::fs::File>>,
    file_path: String,
}

impl FileEnrichOutputAdapter {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(file_path);
        let dir = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        
        info!("Creating enrich output file: {}", file_path);
        
        let file_writer = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(file_path)?
        );
        
        Ok(Self {
            file_writer: Mutex::new(file_writer),
            file_path: file_path.to_string(),
        })
    }
    
    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}

#[async_trait::async_trait]
impl EnrichOutputPort for FileEnrichOutputAdapter {
    async fn write_enriched_record(&self, record: &EnrichedRecord) -> anyhow::Result<()> {
        let json_line = serde_json::to_string(record)?;
        
        // Use mutex to ensure thread-safe writing
        let mut writer = self.file_writer.lock().unwrap();
        writeln!(writer, "{}", json_line)?;
        writer.flush()?;
        
        Ok(())
    }
}
