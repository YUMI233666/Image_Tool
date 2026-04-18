use crate::core::file_discovery::discover_files;
use crate::core::output_resolver::plan_output_paths;
use crate::core::processor::{ProcessContext, ProcessError, ProcessStatus};
use crate::core::registry::ProcessorRegistry;
use crate::core::report::{BatchItemReport, BatchItemStatus, BatchJobReport};
use chrono::Utc;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobRequest {
  pub job_id: String,
  pub processor_id: String,
  pub input_paths: Vec<PathBuf>,
  pub output_dir: PathBuf,
  #[serde(default)]
  pub params: Value,
  pub max_concurrency: Option<usize>,
  #[serde(default)]
  pub include_subdirectories: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressPayload {
  pub job_id: String,
  pub processed: u64,
  pub total: u64,
  pub succeeded: u64,
  pub failed: u64,
  pub skipped: u64,
  pub cancelled: u64,
  pub current_file: String,
  pub status: BatchItemStatus,
  pub message: String,
}

fn default_parallelism() -> usize {
  std::thread::available_parallelism()
    .map(|count| count.get().saturating_sub(1).max(1))
    .unwrap_or(1)
}

fn preferred_extension_for_request(processor_id: &str, params: &Value) -> Option<String> {
  if processor_id != "format-convert" {
    return None;
  }

  let target = params.get("targetFormat")?.as_str()?.trim().to_ascii_lowercase();
  if target.is_empty() {
    return None;
  }

  match target.as_str() {
    "jpeg" => Some("jpg".to_string()),
    "png" | "jpg" | "webp" | "bmp" | "tiff" => Some(target),
    _ => None,
  }
}

pub fn run_batch_job(
  request: BatchJobRequest,
  registry: &ProcessorRegistry,
  should_cancel: &(dyn Fn() -> bool + Send + Sync),
  on_progress: &(dyn Fn(BatchProgressPayload) + Send + Sync),
) -> Result<BatchJobReport, ProcessError> {
  let processor = registry
    .get(&request.processor_id)
    .ok_or_else(|| ProcessError::Validation("处理器不存在或未注册。".to_string()))?;

  processor.validate(&request.params)?;

  let input_files = discover_files(
    &request.input_paths,
    &request.processor_id,
    request.include_subdirectories,
  )?;
  let preferred_extension =
    preferred_extension_for_request(&request.processor_id, &request.params);
  let planned_paths =
    plan_output_paths(&input_files, &request.output_dir, preferred_extension.as_deref())?;

  let total = planned_paths.len() as u64;
  let mut report = BatchJobReport::new(
    request.job_id.clone(),
    request.processor_id.clone(),
    Utc::now().to_rfc3339(),
    total,
  );

  if total == 0 {
    report.finished_at = Utc::now().to_rfc3339();
    return Ok(report);
  }

  let max_concurrency = request.max_concurrency.unwrap_or_else(default_parallelism).max(1);
  let thread_pool = rayon::ThreadPoolBuilder::new()
    .num_threads(max_concurrency)
    .build()
    .map_err(|err| ProcessError::Internal(format!("初始化并发线程池失败: {err}")))?;

  let succeeded = AtomicU64::new(0);
  let failed = AtomicU64::new(0);
  let skipped = AtomicU64::new(0);
  let cancelled = AtomicU64::new(0);
  let processed = AtomicU64::new(0);
  let item_reports: Arc<Mutex<Vec<BatchItemReport>>> = Arc::new(Mutex::new(Vec::new()));
  let shared_params = request.params.clone();

  thread_pool.install(|| {
    planned_paths.par_iter().for_each(|(input_path, output_path)| {
      let start = Instant::now();
      let (status, message, final_output_path) = if should_cancel() {
        cancelled.fetch_add(1, Ordering::Relaxed);
        (
          BatchItemStatus::Cancelled,
          "任务已取消。".to_string(),
          None,
        )
      } else {
        let context = ProcessContext {
          processor_id: request.processor_id.clone(),
          input_path: input_path.clone(),
          output_path: output_path.clone(),
          params: shared_params.clone(),
        };

        match processor.process(&context) {
          Ok(result) => match result.status {
            ProcessStatus::Success => {
              succeeded.fetch_add(1, Ordering::Relaxed);
              (
                BatchItemStatus::Success,
                result.message,
                result
                  .output_path
                  .map(|next_output| next_output.to_string_lossy().to_string())
                  .or_else(|| Some(output_path.to_string_lossy().to_string())),
              )
            }
            ProcessStatus::Skipped => {
              skipped.fetch_add(1, Ordering::Relaxed);
              (
                BatchItemStatus::Skipped,
                result.message,
                result
                  .output_path
                  .map(|next_output| next_output.to_string_lossy().to_string()),
              )
            }
          },
          Err(err) => {
            failed.fetch_add(1, Ordering::Relaxed);
            (BatchItemStatus::Failed, err.user_message(), None)
          }
        }
      };

      let processed_now = processed.fetch_add(1, Ordering::Relaxed) + 1;
      let event = BatchProgressPayload {
        job_id: request.job_id.clone(),
        processed: processed_now,
        total,
        succeeded: succeeded.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
        skipped: skipped.load(Ordering::Relaxed),
        cancelled: cancelled.load(Ordering::Relaxed),
        current_file: input_path.to_string_lossy().to_string(),
        status: status.clone(),
        message: message.clone(),
      };

      if let Ok(mut guard) = item_reports.lock() {
        guard.push(BatchItemReport {
          input_path: input_path.to_string_lossy().to_string(),
          output_path: final_output_path,
          status,
          message,
          duration_ms: start.elapsed().as_millis() as u64,
        });
      }

      on_progress(event);
    });
  });

  let mut items = match Arc::try_unwrap(item_reports) {
    Ok(mutex) => mutex.into_inner().map_err(|_| {
      ProcessError::Internal("批处理记录回收失败，锁已损坏。".to_string())
    })?,
    Err(shared) => shared
      .lock()
      .map_err(|_| ProcessError::Internal("批处理记录读取失败。".to_string()))?
      .clone(),
  };

  items.sort_by(|left, right| left.input_path.cmp(&right.input_path));

  report.succeeded = succeeded.load(Ordering::Relaxed);
  report.failed = failed.load(Ordering::Relaxed);
  report.skipped = skipped.load(Ordering::Relaxed);
  report.cancelled = cancelled.load(Ordering::Relaxed);
  report.finished_at = Utc::now().to_rfc3339();
  report.items = items;

  Ok(report)
}
