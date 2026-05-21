use crate::core::processor::ProcessError;
use std::path::{Path, PathBuf};
use tauri::api::process::Command;

pub fn run_upscale_sidecar(
  input_path: &Path,
  output_path: &Path,
  scale: u8,
  denoise_level: u8,
) -> Result<PathBuf, ProcessError> {
  ensure_models_dir()?;

  if let Some(parent) = output_path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  let input_arg = input_path.to_string_lossy().to_string();
  let output_arg = output_path.to_string_lossy().to_string();
  let denoise_arg = denoise_level.to_string();
  let scale_arg = scale.to_string();

  let command = Command::new_sidecar("realcugan-ncnn-vulkan")
    .map_err(|err| {
      ProcessError::Internal(format!("未找到超分引擎，请确认 sidecar 已配置: {err}"))
    })?;

  let output = command
    .args([
      "-i",
      input_arg.as_str(),
      "-o",
      output_arg.as_str(),
      "-n",
      denoise_arg.as_str(),
      "-s",
      scale_arg.as_str(),
      "-m",
      "models-se",
    ])
    .output()
    .map_err(|err| ProcessError::Internal(format!("执行超分失败: {err}")))?;

  if !output.status.success() {
    let stderr = output.stderr.trim().to_string();
    let message = if stderr.is_empty() {
      "超分失败，请检查模型文件是否正确放置。".to_string()
    } else {
      stderr
    };
    return Err(ProcessError::Internal(message));
  }

  Ok(output_path.to_path_buf())
}

fn ensure_models_dir() -> Result<(), ProcessError> {
  let exe_path = std::env::current_exe()
    .map_err(|err| ProcessError::Internal(format!("无法定位程序路径: {err}")))?;
  let exe_dir = exe_path
    .parent()
    .ok_or_else(|| ProcessError::Internal("无法定位程序目录。".to_string()))?;
  let target_models = exe_dir.join("models-se");

  if target_models.exists() {
    return Ok(());
  }

  let Some(source_models) = find_models_source(exe_dir) else {
    return Err(ProcessError::Internal(
      "未找到 models-se，请将模型目录放在可执行文件同级目录，或放在 src-tauri/binaries/models-se。"
        .to_string(),
    ));
  };

  copy_dir_recursive(&source_models, &target_models)?;

  if !target_models.exists() {
    return Err(ProcessError::Internal(
      "模型目录复制失败，请检查 models-se 目录是否完整。".to_string(),
    ));
  }

  Ok(())
}

fn find_models_source(exe_dir: &Path) -> Option<PathBuf> {
  let resource_candidates = [
    exe_dir.join("resources").join("binaries").join("models-se"),
    exe_dir.join("resources").join("models-se"),
  ];

  for candidate in resource_candidates {
    if candidate.exists() {
      return Some(candidate);
    }
  }

  for ancestor in exe_dir.ancestors().take(5) {
    let src_tauri = ancestor.join("src-tauri").join("binaries").join("models-se");
    if src_tauri.exists() {
      return Some(src_tauri);
    }

    let binaries = ancestor.join("binaries").join("models-se");
    if binaries.exists() {
      return Some(binaries);
    }
  }

  None
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), ProcessError> {
  std::fs::create_dir_all(target)?;

  for entry in std::fs::read_dir(source)? {
    let entry = entry?;
    let file_type = entry.file_type()?;
    let src_path = entry.path();
    let dest_path = target.join(entry.file_name());

    if file_type.is_dir() {
      copy_dir_recursive(&src_path, &dest_path)?;
    } else {
      std::fs::copy(&src_path, &dest_path)?;
    }
  }

  Ok(())
}
