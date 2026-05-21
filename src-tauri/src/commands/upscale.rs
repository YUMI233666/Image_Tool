use crate::core::processor::ProcessError;
use crate::core::upscale_runner::run_upscale_sidecar;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

trait AppHandleEmitExt {
  fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) -> Result<(), tauri::Error>;
}

impl AppHandleEmitExt for tauri::AppHandle {
  fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) -> Result<(), tauri::Error> {
    self.emit_all(event, payload)
  }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleAnimeRequest {
  pub input_path: String,
  pub output_path: String,
  pub scale: u8,
  pub denoise_level: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleAnimeResponse {
  pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleStartPayload {
  pub input: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleProgressPayload {
  pub input: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleCompletePayload {
  pub input: String,
  pub output: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleErrorPayload {
  pub input: String,
  pub error: String,
}

fn validate_request(request: &UpscaleAnimeRequest) -> Result<(), String> {
  if request.input_path.trim().is_empty() {
    return Err("输入路径不能为空。".to_string());
  }

  if request.output_path.trim().is_empty() {
    return Err("输出路径不能为空。".to_string());
  }

  if request.scale != 2 && request.scale != 4 {
    return Err("倍率仅支持 2x 或 4x。".to_string());
  }

  if !(1..=3).contains(&request.denoise_level) {
    return Err("降噪等级仅支持 1-3。".to_string());
  }

  Ok(())
}

fn input_label(path: &PathBuf) -> String {
  path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("input")
    .to_string()
}

#[tauri::command]
pub async fn upscale_anime(
  app_handle: tauri::AppHandle,
  request: UpscaleAnimeRequest,
) -> Result<UpscaleAnimeResponse, String> {
  validate_request(&request)?;

  let input_path = PathBuf::from(request.input_path.trim());
  let output_path = PathBuf::from(request.output_path.trim());
  let label = input_label(&input_path);

  let _ = app_handle.emit("upscale-start", UpscaleStartPayload {
    input: label.clone(),
  });

  let app_for_progress = app_handle.clone();
  let input_for_event = label.clone();
  let scale = request.scale;
  let denoise_level = request.denoise_level;

  let run_result = tauri::async_runtime::spawn_blocking(move || {
    let _ = app_for_progress.emit(
      "upscale-progress",
      UpscaleProgressPayload {
        input: input_for_event,
        status: "processing".to_string(),
      },
    );

    run_upscale_sidecar(&input_path, &output_path, scale, denoise_level)
  })
  .await
  .map_err(|err| format!("超分任务执行失败: {err}"))?;

  match run_result {
    Ok(output_path) => {
      let output_payload = output_path.to_string_lossy().to_string();
      let _ = app_handle.emit(
        "upscale-complete",
        UpscaleCompletePayload {
          input: label.clone(),
          output: output_payload.clone(),
        },
      );
      Ok(UpscaleAnimeResponse {
        output_path: output_payload,
      })
    }
    Err(err) => {
      let message = match err {
        ProcessError::Validation(msg)
        | ProcessError::Unsupported(msg)
        | ProcessError::Internal(msg) => msg,
        other => other.user_message(),
      };
      let _ = app_handle.emit(
        "upscale-error",
        UpscaleErrorPayload {
          input: label,
          error: message.clone(),
        },
      );
      Err(message)
    }
  }
}
