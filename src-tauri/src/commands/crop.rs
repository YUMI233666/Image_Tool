use crate::core::crop_runner::{crop_image_file, CropRect};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropImageRequest {
  pub input_path: String,
  pub output_path: String,
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CropImageResponse {
  pub output_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CropStartPayload {
  pub input: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CropCompletePayload {
  pub input: String,
  pub output: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CropErrorPayload {
  pub input: String,
  pub error: String,
}

fn validate_request(request: &CropImageRequest) -> Result<(), String> {
  if request.input_path.trim().is_empty() {
    return Err("输入路径不能为空。".to_string());
  }

  if request.output_path.trim().is_empty() {
    return Err("输出路径不能为空。".to_string());
  }

  if request.width == 0 || request.height == 0 {
    return Err("裁剪宽高必须大于 0。".to_string());
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
pub async fn crop_image(
  app_handle: tauri::AppHandle,
  request: CropImageRequest,
) -> Result<CropImageResponse, String> {
  validate_request(&request)?;

  let input_path = PathBuf::from(request.input_path.trim());
  let output_path = PathBuf::from(request.output_path.trim());
  let label = input_label(&input_path);
  let rect = CropRect {
    x: request.x,
    y: request.y,
    width: request.width,
    height: request.height,
  };

  let input_path_for_task = input_path.clone();
  let output_path_for_task = output_path.clone();

  let _ = app_handle.emit_all(
    "crop-start",
    CropStartPayload {
      input: label.clone(),
    },
  );

  let app_for_event = app_handle.clone();
  let label_for_event = label.clone();

  let run_result = tauri::async_runtime::spawn_blocking(move || {
    crop_image_file(&input_path_for_task, &output_path_for_task, rect)
  })
  .await
  .map_err(|err| format!("裁剪任务执行失败: {err}"))?;

  match run_result {
    Ok(_) => {
      let output_payload = output_path.to_string_lossy().to_string();
      let _ = app_for_event.emit_all(
        "crop-complete",
        CropCompletePayload {
          input: label_for_event,
          output: output_payload.clone(),
        },
      );
      Ok(CropImageResponse {
        output_path: output_payload,
      })
    }
    Err(err) => {
      let message = err.user_message();
      let _ = app_for_event.emit_all(
        "crop-error",
        CropErrorPayload {
          input: label_for_event,
          error: message.clone(),
        },
      );
      Err(message)
    }
  }
}
