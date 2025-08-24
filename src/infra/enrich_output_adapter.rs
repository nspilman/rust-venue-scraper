use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use crate::app::ports::EnrichOutputPort;
use crate::pipeline::processing::enrich::EnrichedRecord;

/// File-based adapter for writing enriched records to NDJSON files
pub struct FileEnrichOutputAdapter {
    pub output_dir: PathBuf,
}

impl FileEnrichOutputAdapter {
    pub fn new(output_dir: PathBuf) -> Self { Self { output_dir } }

    fn get_output_path(&self, record: &EnrichedRecord) -> PathBuf {
        let mut path = self.output_dir.clone();
        let date = record.enriched_at;
        path.push("enriched");
        path.push(format!("year={}", date.format("%Y")));
        path.push(format!("month={}", date.format("%m")));
        path.push(format!("day={}", date.format("%d")));
        let filename = format!("enriched-{}.ndjson", date.format("%Y%m%d"));
        path.push(filename);
        path
    }

    async fn ensure_dir(&self, file_path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = file_path.parent() { tokio::fs::create_dir_all(parent).await?; }
        Ok(())
    }
}

#[async_trait]
impl EnrichOutputPort for FileEnrichOutputAdapter {
    async fn write_enriched_record(&self, record: &EnrichedRecord) -> anyhow::Result<()> {
        let path = self.get_output_path(record);
        self.ensure_dir(&path).await?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path).await?;
        let line = serde_json::to_string(record)? + "\n";
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        debug!("wrote enriched record to {:?}", path);
        Ok(())
    }
}
