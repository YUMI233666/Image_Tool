use crate::core::processor::ProcessError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BatchItemStatus {
  Success,
  Failed,
  Skipped,
  Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItemReport {
  pub input_path: String,
  pub output_path: Option<String>,
  pub status: BatchItemStatus,
  pub message: String,
  pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobReport {
  pub job_id: String,
  pub processor_id: String,
  pub started_at: String,
  pub finished_at: String,
  pub total: u64,
  pub succeeded: u64,
  pub failed: u64,
  pub skipped: u64,
  pub cancelled: u64,
  pub report_path: Option<String>,
  pub items: Vec<BatchItemReport>,
}

impl BatchJobReport {
  pub fn new(job_id: String, processor_id: String, started_at: String, total: u64) -> Self {
    Self {
      job_id,
      processor_id,
      started_at,
      finished_at: String::new(),
      total,
      succeeded: 0,
      failed: 0,
      skipped: 0,
      cancelled: 0,
      report_path: None,
      items: Vec::new(),
    }
  }
}

pub fn write_report_to_file(
  report: &BatchJobReport,
  output_dir: &Path,
) -> Result<PathBuf, ProcessError> {
  std::fs::create_dir_all(output_dir)?;

  let report_path = output_dir.join(format!("batch-report-{}.json", report.job_id));
  let payload = serde_json::to_string_pretty(report)?;
  std::fs::write(&report_path, payload)?;

  Ok(report_path)
}
