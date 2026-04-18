use crate::core::batch_job_runner::{run_batch_job, BatchJobRequest};
use crate::core::processor::ProcessorDescriptor;
use crate::core::registry::ProcessorRegistry;
use crate::core::report::{write_report_to_file, BatchJobReport};
use dashmap::DashSet;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;
use uuid::Uuid;

static PROCESSOR_REGISTRY: Lazy<ProcessorRegistry> = Lazy::new(ProcessorRegistry::default_registry);
static CANCELLED_JOBS: Lazy<DashSet<String>> = Lazy::new(DashSet::new);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartBatchJobRequest {
  pub processor_id: String,
  pub input_paths: Vec<String>,
  pub output_dir: String,
  #[serde(default)]
  pub params: Option<Value>,
  #[serde(default)]
  pub max_concurrency: Option<usize>,
  #[serde(default)]
  pub include_subdirectories: Option<bool>,
  #[serde(default)]
  pub write_report: Option<bool>,
}

#[tauri::command]
pub fn list_processors() -> Vec<ProcessorDescriptor> {
  PROCESSOR_REGISTRY.descriptors()
}

#[tauri::command]
pub async fn start_batch_job(
  app: tauri::AppHandle,
  request: StartBatchJobRequest,
) -> Result<BatchJobReport, String> {
  if request.input_paths.is_empty() {
    return Err("未提供输入路径。".to_string());
  }

  if request.output_dir.trim().is_empty() {
    return Err("输出目录不能为空。".to_string());
  }

  let job_id = Uuid::new_v4().to_string();
  let include_subdirectories = request.include_subdirectories.unwrap_or(true);

  let batch_request = BatchJobRequest {
    job_id: job_id.clone(),
    processor_id: request.processor_id.clone(),
    input_paths: request.input_paths.iter().map(PathBuf::from).collect(),
    output_dir: PathBuf::from(&request.output_dir),
    params: request.params.unwrap_or(Value::Null),
    max_concurrency: request.max_concurrency,
    include_subdirectories,
  };

  let app_for_progress = app.clone();
  let cancel_key = job_id.clone();

  let run_result = tauri::async_runtime::spawn_blocking(move || {
    run_batch_job(
      batch_request,
      &PROCESSOR_REGISTRY,
      &|| CANCELLED_JOBS.contains(cancel_key.as_str()),
      &|payload| {
        let _ = app_for_progress.emit_all("batch-progress", payload);
      },
    )
  })
  .await
  .map_err(|err| format!("批处理线程执行异常: {err}"))?;

  CANCELLED_JOBS.remove(job_id.as_str());

  let mut report = run_result.map_err(|err| err.user_message())?;

  if request.write_report.unwrap_or(true) {
    let report_path = write_report_to_file(&report, &PathBuf::from(&request.output_dir))
      .map_err(|err| err.user_message())?;
    report.report_path = Some(report_path.to_string_lossy().to_string());
  }

  let _ = app.emit_all("batch-complete", report.clone());

  Ok(report)
}

#[tauri::command]
pub fn cancel_batch_job(job_id: String) -> Result<(), String> {
  if job_id.trim().is_empty() {
    return Err("jobId 不能为空。".to_string());
  }

  CANCELLED_JOBS.insert(job_id);
  Ok(())
}

#[tauri::command]
pub fn open_path_in_system(app: tauri::AppHandle, path: String) -> Result<(), String> {
  let trimmed = path.trim();
  if trimmed.is_empty() {
    return Err("路径不能为空。".to_string());
  }

  let target = PathBuf::from(trimmed);
  if !target.exists() {
    return Err(format!("路径不存在: {trimmed}"));
  }

  let shell_scope = app.shell_scope();
  if let Err(primary_error) = tauri::api::shell::open(&shell_scope, trimmed, None) {
    #[cfg(target_os = "windows")]
    {
      Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(trimmed)
        .spawn()
        .map_err(|fallback_error| {
          format!(
            "系统打开失败。主错误: {primary_error}; 回退错误: {fallback_error}"
          )
        })?;
      return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
      return Err(format!("系统打开失败: {primary_error}"));
    }
  }

  Ok(())
}
