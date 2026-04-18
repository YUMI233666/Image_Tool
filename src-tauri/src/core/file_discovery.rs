use crate::core::processor::ProcessError;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn supported_extensions(processor_id: &str) -> &'static [&'static str] {
  match processor_id {
    "trim-transparent" => &["png"],
    "format-convert" | "compress" | "repair" => {
      &["png", "jpg", "jpeg", "webp", "bmp", "tiff"]
    }
    _ => &["png"],
  }
}

fn has_supported_extension(path: &Path, extensions: &[&str]) -> bool {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| {
      let normalized = ext.to_ascii_lowercase();
      extensions.iter().any(|candidate| *candidate == normalized)
    })
    .unwrap_or(false)
}

pub fn discover_files(
  input_paths: &[PathBuf],
  processor_id: &str,
  include_subdirectories: bool,
) -> Result<Vec<PathBuf>, ProcessError> {
  if input_paths.is_empty() {
    return Err(ProcessError::Validation("未提供任何输入路径。".to_string()));
  }

  let extensions = supported_extensions(processor_id);
  let mut files = BTreeSet::new();

  for input in input_paths {
    if input.is_file() {
      if has_supported_extension(input, extensions) {
        files.insert(input.to_path_buf());
      }
      continue;
    }

    if input.is_dir() {
      let mut walker = WalkDir::new(input).min_depth(1);
      if !include_subdirectories {
        walker = walker.max_depth(1);
      }

      for entry in walker.into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
          continue;
        }

        let path = entry.path();
        if has_supported_extension(path, extensions) {
          files.insert(path.to_path_buf());
        }
      }
    }
  }

  if files.is_empty() {
    return Err(ProcessError::Validation(
      "未找到可处理的图片文件。".to_string(),
    ));
  }

  Ok(files.into_iter().collect())
}
