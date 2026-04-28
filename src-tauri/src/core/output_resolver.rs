use crate::core::processor::ProcessError;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const DEFAULT_RENAME_TEMPLATE: &str = "{name}_{index}";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RenameMode {
  #[default]
  Custom,
  Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameConfig {
  #[serde(default)]
  pub enabled: bool,
  #[serde(default)]
  pub mode: RenameMode,
  #[serde(default)]
  pub custom_name: Option<String>,
  #[serde(default)]
  pub template: Option<String>,
  #[serde(default = "default_start_index")]
  pub start_index: u32,
  #[serde(default)]
  pub index_padding: u8,
}

impl Default for RenameConfig {
  fn default() -> Self {
    Self {
      enabled: false,
      mode: RenameMode::Custom,
      custom_name: None,
      template: None,
      start_index: default_start_index(),
      index_padding: 0,
    }
  }
}

fn default_start_index() -> u32 {
  1
}

fn normalize_extension(extension: &str) -> Option<String> {
  let trimmed = extension.trim().trim_start_matches('.').to_ascii_lowercase();
  if trimmed.is_empty() {
    return None;
  }

  Some(trimmed)
}

fn sanitize_base_name(raw: &str) -> String {
  let mut cleaned = String::with_capacity(raw.len());

  for ch in raw.chars() {
    let normalized = match ch {
      '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
      _ => ch,
    };
    cleaned.push(normalized);
  }

  let trimmed = cleaned.trim().trim_matches('.');
  if trimmed.is_empty() {
    return "output".to_string();
  }

  let truncated = trimmed.chars().take(120).collect::<String>();
  if truncated.is_empty() {
    "output".to_string()
  } else {
    truncated
  }
}

fn strip_matching_extension(rendered: &str, extension: &str) -> String {
  if extension.is_empty() {
    return rendered.to_string();
  }

  let suffix = format!(".{extension}");
  if rendered.to_ascii_lowercase().ends_with(&suffix.to_ascii_lowercase()) {
    rendered
      .chars()
      .take(rendered.chars().count().saturating_sub(suffix.chars().count()))
      .collect::<String>()
  } else {
    rendered.to_string()
  }
}

fn format_index(value: u32, padding: u8) -> String {
  if padding == 0 {
    return value.to_string();
  }

  format!("{value:0width$}", width = padding as usize)
}

fn render_template(
  template: &str,
  source_name: &str,
  extension: &str,
  index: u32,
  padding: u8,
) -> String {
  let now = Local::now();
  let date = now.format("%Y%m%d").to_string();
  let time = now.format("%H%M%S").to_string();
  let formatted_index = format_index(index, padding);

  template
    .replace("{name}", source_name)
    .replace("{index}", formatted_index.as_str())
    .replace("{date}", date.as_str())
    .replace("{time}", time.as_str())
    .replace("{ext}", extension)
}

fn resolve_base_name(
  input: &Path,
  list_index: usize,
  extension: &str,
  rename_config: Option<&RenameConfig>,
) -> String {
  let source_name = input
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("output");

  let Some(config) = rename_config else {
    return sanitize_base_name(source_name);
  };

  if !config.enabled {
    return sanitize_base_name(source_name);
  }

  let serial = config.start_index.saturating_add(list_index as u32);
  let raw_base = match config.mode {
    RenameMode::Custom => config.custom_name.as_deref().unwrap_or(source_name).to_string(),
    RenameMode::Template => {
      let template = config.template.as_deref().unwrap_or(DEFAULT_RENAME_TEMPLATE);
      let rendered = render_template(
        template,
        source_name,
        extension,
        serial,
        config.index_padding,
      );
      strip_matching_extension(rendered.as_str(), extension)
    }
  };

  sanitize_base_name(raw_base.as_str())
}

pub fn plan_output_paths(
  input_files: &[PathBuf],
  output_dir: &Path,
  preferred_extension: Option<&str>,
  rename_config: Option<&RenameConfig>,
) -> Result<Vec<(PathBuf, PathBuf)>, ProcessError> {
  if input_files.is_empty() {
    return Ok(Vec::new());
  }

  std::fs::create_dir_all(output_dir)?;

  let mut used_paths = HashSet::new();
  let mut planned = Vec::with_capacity(input_files.len());

  for (list_index, input) in input_files.iter().enumerate() {
    let extension_no_dot = match preferred_extension.and_then(normalize_extension) {
      Some(ext) => ext,
      None => input
        .extension()
        .and_then(|e| e.to_str())
        .and_then(normalize_extension)
        .unwrap_or_default(),
    };
    let stem = resolve_base_name(input, list_index, extension_no_dot.as_str(), rename_config);

    let extension = if extension_no_dot.is_empty() {
      String::new()
    } else {
      format!(".{extension_no_dot}")
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
