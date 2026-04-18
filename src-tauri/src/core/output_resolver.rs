use crate::core::processor::ProcessError;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn normalize_extension(extension: &str) -> Option<String> {
  let trimmed = extension.trim().trim_start_matches('.').to_ascii_lowercase();
  if trimmed.is_empty() {
    return None;
  }

  Some(trimmed)
}

pub fn plan_output_paths(
  input_files: &[PathBuf],
  output_dir: &Path,
  preferred_extension: Option<&str>,
) -> Result<Vec<(PathBuf, PathBuf)>, ProcessError> {
  if input_files.is_empty() {
    return Ok(Vec::new());
  }

  std::fs::create_dir_all(output_dir)?;

  let mut used_paths = HashSet::new();
  let mut planned = Vec::with_capacity(input_files.len());

  for input in input_files {
    let stem = input
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("output")
      .to_string();

    let extension = match preferred_extension.and_then(normalize_extension) {
      Some(ext) => format!(".{ext}"),
      None => input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default(),
    };

    let mut index = 0u32;
    let output = loop {
      let suffix = if index == 0 {
        String::new()
      } else {
        format!("_{index}")
      };
      let candidate = output_dir.join(format!("{stem}{suffix}{extension}"));

      if !candidate.exists() && !used_paths.contains(&candidate) {
        break candidate;
      }

      index = index.saturating_add(1);
    };

    used_paths.insert(output.clone());
    planned.push((input.clone(), output));
  }

  Ok(planned)
}
