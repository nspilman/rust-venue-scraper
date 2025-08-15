use crate::app::ports::QualityGateOutputPort;
use crate::pipeline::processing::quality_gate::QualityAssessedRecord;
use std::io::{BufWriter, Write};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;
use tracing::info;

/// File-based implementation of QualityGateOutputPort
/// Writes quality-assessed records to NDJSON files
pub struct FileQualityGateOutputAdapter {
    file_writer: Mutex<BufWriter<std::fs::File>>,
    file_path: String,
}

impl FileQualityGateOutputAdapter {
    pub fn new(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(file_path);
        let dir = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        
        info!("Creating quality gate output file: {}", file_path);
        
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
impl QualityGateOutputPort for FileQualityGateOutputAdapter {
    async fn write_quality_assessed_record(&self, record: &QualityAssessedRecord) -> anyhow::Result<()> {
        let json_line = serde_json::to_string(record)?;
        
        // Use mutex to ensure thread-safe writing
        let mut writer = self.file_writer.lock().unwrap();
        writeln!(writer, "{}", json_line)?;
        writer.flush()?;
        
        Ok(())
    }
}
