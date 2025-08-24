use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::debug;

use crate::app::ports::QualityGateOutputPort;
use crate::pipeline::processing::quality_gate::QualityAssessedRecord;

/// File-based adapter for writing quality-assessed records to NDJSON files
/// Partitioned into accepted and quarantined subfolders with date-based directories
pub struct FileQualityGateOutputAdapter {
    pub output_dir: PathBuf,
    pub partition: QualityPartition,
}

#[derive(Clone, Copy)]
pub enum QualityPartition { Accepted, Quarantined }

impl FileQualityGateOutputAdapter {
    pub fn new(output_dir: PathBuf, partition: QualityPartition) -> Self {
        Self { output_dir, partition }
    }

    fn get_output_path(&self, record: &QualityAssessedRecord) -> PathBuf {
        let mut path = self.output_dir.clone();
        let date = record.assessed_at;
        path.push("quality");
        path.push(match self.partition { QualityPartition::Accepted => "accepted", QualityPartition::Quarantined => "quarantined" });
        path.push(format!("year={}", date.format("%Y")));
        path.push(format!("month={}", date.format("%m")));
        path.push(format!("day={}", date.format("%d")));
        let filename = format!("quality-{}.ndjson", date.format("%Y%m%d"));
        path.push(filename);
        path
    }

    async fn ensure_dir(&self, file_path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = file_path.parent() { tokio::fs::create_dir_all(parent).await?; }
        Ok(())
    }
}

#[async_trait]
impl QualityGateOutputPort for FileQualityGateOutputAdapter {
    async fn write_quality_assessed_record(&self, record: &QualityAssessedRecord) -> anyhow::Result<()> {
        let path = self.get_output_path(record);
        self.ensure_dir(&path).await?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path).await?;
        let line = serde_json::to_string(record)? + "\n";
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        debug!("wrote quality record to {:?}", path);
        Ok(())
    }
}
