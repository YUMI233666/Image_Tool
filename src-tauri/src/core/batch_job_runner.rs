use crate::core::file_discovery::discover_files;
use crate::core::output_resolver::{plan_output_paths, RenameConfig};
use crate::core::processor::{ProcessContext, ProcessError, ProcessStatus};
use crate::core::registry::ProcessorRegistry;
use crate::core::report::{
  BatchItemReport,
  BatchItemStatus,
  BatchJobReport,
  BatchStepReport,
};
use chrono::Utc;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStepRequest {
  #[serde(default)]
  pub step_id: String,
  pub processor_id: String,
  #[serde(default)]
  pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobRequest {
  pub job_id: String,
  pub processor_id: String,
  pub input_paths: Vec<PathBuf>,
  pub output_dir: PathBuf,
  #[serde(default)]
  pub params: Value,
  #[serde(default)]
  pub workflow_steps: Vec<WorkflowStepRequest>,
  #[serde(default)]
  pub rename_config: Option<RenameConfig>,
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
  pub current_step_processor_id: Option<String>,
  pub current_step_index: Option<u32>,
  pub current_step_total: Option<u32>,
  pub current_step_message: Option<String>,
}

fn default_parallelism() -> usize {
  std::thread::available_parallelism()
    .map(|count| count.get().saturating_sub(1).max(1))
    .unwrap_or(1)
}

fn preferred_extension_for_step(processor_id: &str, params: &Value) -> Option<String> {
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

fn preferred_extension_for_steps(steps: &[WorkflowStepRequest]) -> Option<String> {
  steps
    .iter()
    .rev()
    .find_map(|step| preferred_extension_for_step(step.processor_id.as_str(), &step.params))
}

fn resolve_effective_steps(request: &BatchJobRequest) -> Vec<WorkflowStepRequest> {
  if !request.workflow_steps.is_empty() {
    return request.workflow_steps.clone();
  }

  vec![WorkflowStepRequest {
    step_id: "legacy-step-1".to_string(),
    processor_id: request.processor_id.clone(),
    params: request.params.clone(),
  }]
}

fn extension_for_path(path: &Path) -> String {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
    .filter(|ext| !ext.is_empty())
    .unwrap_or_else(|| "png".to_string())
}

fn sanitize_fragment(raw: &str) -> String {
  let filtered = raw
    .chars()
    .map(|ch| match ch {
      'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
      _ => '_',
    })
    .collect::<String>();

  let trimmed = filtered.trim_matches('_');
  if trimmed.is_empty() {
    "item".to_string()
  } else {
    trimmed.chars().take(40).collect::<String>()
  }
}

fn build_temp_output_path(
  temp_root: &Path,
  file_index: usize,
  step_index: usize,
  input_path: &Path,
  step: &WorkflowStepRequest,
) -> PathBuf {
  let extension = preferred_extension_for_step(step.processor_id.as_str(), &step.params)
    .unwrap_or_else(|| extension_for_path(input_path));
  let source_stem = input_path
    .file_stem()
    .and_then(|stem| stem.to_str())
    .unwrap_or("item");
  let source_fragment = sanitize_fragment(source_stem);
  temp_root.join(format!(
    "item-{file_index}-step-{step_index}-{source_fragment}.{extension}"
  ))
}

fn remove_temp_files(paths: &[PathBuf]) {
  for path in paths {
    let _ = std::fs::remove_file(path);
  }
}

fn default_final_message(status: &BatchItemStatus) -> String {
  match status {
    BatchItemStatus::Running => "任务处理中。".to_string(),
    BatchItemStatus::Success => "处理完成。".to_string(),
    BatchItemStatus::Failed => "处理失败。".to_string(),
    BatchItemStatus::Skipped => "已跳过。".to_string(),
    BatchItemStatus::Cancelled => "任务已取消。".to_string(),
  }
}

pub fn run_batch_job(
  request: BatchJobRequest,
  registry: &ProcessorRegistry,
  should_cancel: &(dyn Fn() -> bool + Send + Sync),
  on_progress: &(dyn Fn(BatchProgressPayload) + Send + Sync),
) -> Result<BatchJobReport, ProcessError> {
  on_progress(BatchProgressPayload {
    job_id: request.job_id.clone(),
    processed: 0,
    total: 0,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    cancelled: 0,
    current_file: String::new(),
    status: BatchItemStatus::Running,
    message: "正在扫描输入文件...".to_string(),
    current_step_processor_id: None,
    current_step_index: None,
    current_step_total: None,
    current_step_message: Some("准备阶段".to_string()),
  });

  let steps = resolve_effective_steps(&request);
  if steps.is_empty() {
    return Err(ProcessError::Validation(
      "工作流步骤不能为空。".to_string(),
    ));
  }

  for step in &steps {
    if step.processor_id.trim().is_empty() {
      return Err(ProcessError::Validation(
        "工作流存在空处理器标识。".to_string(),
      ));
    }

    let processor = registry.get(step.processor_id.as_str()).ok_or_else(|| {
      ProcessError::Validation(format!("处理器不存在或未注册: {}", step.processor_id))
    })?;

    processor.validate(&step.params)?;
  }

  let discovery_processor_id = steps
    .first()
    .map(|step| step.processor_id.as_str())
    .unwrap_or(request.processor_id.as_str());

  let input_files = discover_files(
    &request.input_paths,
    discovery_processor_id,
    request.include_subdirectories,
  )?;
  let preferred_extension = preferred_extension_for_steps(&steps);
  let planned_paths = plan_output_paths(
    &input_files,
    &request.output_dir,
    preferred_extension.as_deref(),
    request.rename_config.as_ref(),
  )?;

  let total = planned_paths.len() as u64;
  on_progress(BatchProgressPayload {
    job_id: request.job_id.clone(),
    processed: 0,
    total,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    cancelled: 0,
    current_file: String::new(),
    status: BatchItemStatus::Running,
    message: format!("已发现 {total} 个文件，准备开始处理。"),
    current_step_processor_id: None,
    current_step_index: None,
    current_step_total: None,
    current_step_message: Some("准备阶段".to_string()),
  });
  let report_processor_id = if request.workflow_steps.is_empty() {
    request.processor_id.clone()
  } else {
    "workflow".to_string()
  };
  let mut report = BatchJobReport::new(
    request.job_id.clone(),
    report_processor_id,
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
  let shared_steps = Arc::new(steps);
  let shared_job_id = request.job_id.clone();
  let shared_output_dir = request.output_dir.clone();

  thread_pool.install(|| {
    planned_paths
      .par_iter()
      .enumerate()
      .for_each(|(file_index, (input_path, output_path))| {
        let start = Instant::now();
        let mut status = BatchItemStatus::Success;
        let mut message = String::new();
        let mut final_output_path: Option<PathBuf> = None;
        let mut step_reports: Vec<BatchStepReport> =
          Vec::with_capacity(shared_steps.len());
        let mut current_input = input_path.clone();
        let mut temp_outputs: Vec<PathBuf> = Vec::new();

        if should_cancel() {
          status = BatchItemStatus::Cancelled;
          message = "任务已取消。".to_string();
        } else {
          let temp_root = shared_output_dir
            .join(".art-tool-tmp")
            .join(shared_job_id.as_str());

          if let Err(err) = std::fs::create_dir_all(&temp_root) {
            status = BatchItemStatus::Failed;
            message = format!("创建临时目录失败: {err}");
          } else {
            let step_total = shared_steps.len() as u32;

            for (step_zero_index, step) in shared_steps.iter().enumerate() {
              if should_cancel() {
                status = BatchItemStatus::Cancelled;
                message = "任务已取消。".to_string();
                break;
              }

              let step_index = (step_zero_index + 1) as u32;
              on_progress(BatchProgressPayload {
                job_id: shared_job_id.clone(),
                processed: processed.load(Ordering::Relaxed),
                total,
                succeeded: succeeded.load(Ordering::Relaxed),
                failed: failed.load(Ordering::Relaxed),
                skipped: skipped.load(Ordering::Relaxed),
                cancelled: cancelled.load(Ordering::Relaxed),
                current_file: input_path.to_string_lossy().to_string(),
                status: BatchItemStatus::Running,
                message: format!(
                  "正在执行步骤 {step_index}/{step_total}: {}",
                  step.processor_id
                ),
                current_step_processor_id: Some(step.processor_id.clone()),
                current_step_index: Some(step_index),
                current_step_total: Some(step_total),
                current_step_message: Some("开始执行".to_string()),
              });

              let Some(processor) = registry.get(step.processor_id.as_str()) else {
                status = BatchItemStatus::Failed;
                message = format!("处理器不存在或未注册: {}", step.processor_id);
                step_reports.push(BatchStepReport {
                  step_index,
                  step_total,
                  processor_id: step.processor_id.clone(),
                  status: BatchItemStatus::Failed,
                  message: message.clone(),
                  output_path: None,
                  duration_ms: 0,
                });
                break;
              };

              let is_last_step = step_zero_index + 1 == shared_steps.len();
              let expected_output = if is_last_step {
                output_path.clone()
              } else {
                build_temp_output_path(
                  &temp_root,
                  file_index,
                  step_zero_index,
                  input_path,
                  step,
                )
              };

              let step_started = Instant::now();
              let context = ProcessContext {
                processor_id: step.processor_id.clone(),
                input_path: current_input.clone(),
                output_path: expected_output.clone(),
                params: step.params.clone(),
              };

              match processor.process(&context) {
                Ok(result) => match result.status {
                  ProcessStatus::Success => {
                    let next_output =
                      result.output_path.unwrap_or(expected_output.clone());
                    let next_output_string =
                      Some(next_output.to_string_lossy().to_string());

                    step_reports.push(BatchStepReport {
                      step_index,
                      step_total,
                      processor_id: step.processor_id.clone(),
                      status: BatchItemStatus::Success,
                      message: result.message.clone(),
                      output_path: next_output_string,
                      duration_ms: step_started.elapsed().as_millis() as u64,
                    });

                    message = result.message;
                    status = BatchItemStatus::Success;
                    current_input = next_output.clone();
                    final_output_path = Some(next_output.clone());

                    if !is_last_step {
                      temp_outputs.push(next_output);
                    }
                  }
                  ProcessStatus::Skipped => {
                    step_reports.push(BatchStepReport {
                      step_index,
                      step_total,
                      processor_id: step.processor_id.clone(),
                      status: BatchItemStatus::Skipped,
                      message: result.message.clone(),
                      output_path: None,
                      duration_ms: step_started.elapsed().as_millis() as u64,
                    });

                    status = BatchItemStatus::Skipped;
                    message = format!(
                      "步骤 {} 跳过: {}",
                      step.processor_id,
                      result.message
                    );
                    final_output_path = None;
                    break;
                  }
                },
                Err(err) => {
                  let user_message = err.user_message();
                  step_reports.push(BatchStepReport {
                    step_index,
                    step_total,
                    processor_id: step.processor_id.clone(),
                    status: BatchItemStatus::Failed,
                    message: user_message.clone(),
                    output_path: None,
                    duration_ms: step_started.elapsed().as_millis() as u64,
                  });

                  status = BatchItemStatus::Failed;
                  message = format!(
                    "步骤 {} 失败: {}",
                    step.processor_id,
                    user_message
                  );
                  final_output_path = None;
                  break;
                }
              }
            }
          }
        }

        remove_temp_files(&temp_outputs);

        if message.is_empty() {
          message = default_final_message(&status);
        }

        match status {
          BatchItemStatus::Success => {
            succeeded.fetch_add(1, Ordering::Relaxed);
          }
          BatchItemStatus::Failed => {
            failed.fetch_add(1, Ordering::Relaxed);
          }
          BatchItemStatus::Skipped => {
            skipped.fetch_add(1, Ordering::Relaxed);
          }
          BatchItemStatus::Cancelled => {
            cancelled.fetch_add(1, Ordering::Relaxed);
          }
          BatchItemStatus::Running => {}
        }

        let processed_now = processed.fetch_add(1, Ordering::Relaxed) + 1;
        let final_output_path = match status {
          BatchItemStatus::Success => final_output_path,
          _ => None,
        };
        let final_output_string = final_output_path
          .as_ref()
          .map(|path| path.to_string_lossy().to_string());
        let current_step_processor_id =
          step_reports.last().map(|item| item.processor_id.clone());
        let current_step_index = step_reports.last().map(|item| item.step_index);
        let current_step_total = step_reports.last().map(|item| item.step_total);
        let current_step_message =
          step_reports.last().map(|item| item.message.clone());

        let event = BatchProgressPayload {
          job_id: shared_job_id.clone(),
          processed: processed_now,
          total,
          succeeded: succeeded.load(Ordering::Relaxed),
          failed: failed.load(Ordering::Relaxed),
          skipped: skipped.load(Ordering::Relaxed),
          cancelled: cancelled.load(Ordering::Relaxed),
          current_file: input_path.to_string_lossy().to_string(),
          status: status.clone(),
          message: message.clone(),
          current_step_processor_id,
          current_step_index,
          current_step_total,
          current_step_message,
        };

        if let Ok(mut guard) = item_reports.lock() {
          guard.push(BatchItemReport {
            input_path: input_path.to_string_lossy().to_string(),
            output_path: final_output_string,
            status,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
            steps: step_reports,
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
