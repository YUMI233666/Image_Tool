use crate::core::crop_runner::{crop_rgba_image, load_image_rgba, save_rgba_image, CropRect};
use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CropPercentRect {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileCropOverride {
  #[serde(default)]
  skip: bool,
  #[serde(default)]
  rect: Option<CropRect>,
  #[serde(default)]
  percent_rect: Option<CropPercentRect>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CropApplyMode {
  Absolute,
  Percent,
}

impl Default for CropApplyMode {
  fn default() -> Self {
    Self::Percent
  }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManualCropParams {
  #[serde(default)]
  apply_mode: CropApplyMode,
  #[serde(default)]
  default_rect: Option<CropRect>,
  #[serde(default)]
  default_percent_rect: Option<CropPercentRect>,
  #[serde(default)]
  file_overrides: HashMap<String, FileCropOverride>,
}

impl Default for ManualCropParams {
  fn default() -> Self {
    Self {
      apply_mode: CropApplyMode::Percent,
      default_rect: None,
      default_percent_rect: None,
      file_overrides: HashMap::new(),
    }
  }
}

#[derive(Default)]
pub struct ManualCropProcessor;

fn normalize_path_key(raw: &str) -> String {
  raw.replace('\\', "/").to_ascii_lowercase()
}

fn resolve_override<'a>(
  params: &'a ManualCropParams,
  input_path: &Path,
) -> Option<&'a FileCropOverride> {
  if params.file_overrides.is_empty() {
    return None;
  }

  let raw = input_path.to_string_lossy().to_string();
  if let Some(override_item) = params.file_overrides.get(&raw) {
    return Some(override_item);
  }

  let normalized = normalize_path_key(raw.as_str());
  params.file_overrides.get(&normalized)
}

fn validate_percent_rect(rect: &CropPercentRect) -> Result<(), ProcessError> {
  let valid = (0.0..=1.0).contains(&rect.x)
    && (0.0..=1.0).contains(&rect.y)
    && (0.0..=1.0).contains(&rect.width)
    && (0.0..=1.0).contains(&rect.height);

  if !valid {
    return Err(ProcessError::Validation(
      "裁剪百分比必须在 0 到 1 之间。".to_string(),
    ));
  }

  if rect.width <= 0.0 || rect.height <= 0.0 {
    return Err(ProcessError::Validation(
      "裁剪宽高必须大于 0。".to_string(),
    ));
  }

  Ok(())
}

fn percent_to_rect(
  rect: &CropPercentRect,
  width: u32,
  height: u32,
) -> Result<CropRect, ProcessError> {
  validate_percent_rect(rect)?;

  let x = (rect.x * width as f32).floor().max(0.0);
  let y = (rect.y * height as f32).floor().max(0.0);
  let w = (rect.width * width as f32).round().max(1.0);
  let h = (rect.height * height as f32).round().max(1.0);

  let x_u = x as u32;
  let y_u = y as u32;
  let mut w_u = w as u32;
  let mut h_u = h as u32;

  if x_u >= width || y_u >= height {
    return Err(ProcessError::Validation(format!(
      "裁剪起点超出图片尺寸（图片: {width}x{height}）。",
    )));
  }

  if x_u + w_u > width {
    w_u = width - x_u;
  }

  if y_u + h_u > height {
    h_u = height - y_u;
  }

  if w_u == 0 || h_u == 0 {
    return Err(ProcessError::Validation(
      "裁剪宽高必须大于 0。".to_string(),
    ));
  }

  Ok(CropRect {
    x: x_u,
    y: y_u,
    width: w_u,
    height: h_u,
  })
}

fn resolve_crop_rect(
  params: &ManualCropParams,
  override_item: Option<&FileCropOverride>,
  width: u32,
  height: u32,
) -> Result<CropRect, ProcessError> {
  if let Some(item) = override_item {
    if let Some(rect) = item.rect {
      return Ok(rect);
    }

    if let Some(rect) = item.percent_rect.as_ref() {
      return percent_to_rect(rect, width, height);
    }
  }

  match params.apply_mode {
    CropApplyMode::Absolute => params
      .default_rect
      .ok_or_else(|| {
        ProcessError::Validation("未配置裁剪区域，请先设置裁剪框。".to_string())
      }),
    CropApplyMode::Percent => params
      .default_percent_rect
      .as_ref()
      .ok_or_else(|| {
        ProcessError::Validation("未配置裁剪区域，请先设置裁剪框。".to_string())
      })
      .and_then(|rect| percent_to_rect(rect, width, height)),
  }
}

fn copy_original(input: &Path, output: &Path) -> Result<(), ProcessError> {
  if let Some(parent) = output.parent() {
    std::fs::create_dir_all(parent)?;
  }

  std::fs::copy(input, output)?;
  Ok(())
}

impl Processor for ManualCropProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "manual-crop".to_string(),
      display_name: "手动裁剪".to_string(),
      enabled: true,
      notes: "自定义裁剪区域，支持比例锁定与逐张配置。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    if params.is_null() {
      return Ok(());
    }

    let parsed = serde_json::from_value::<ManualCropParams>(params.clone())?;

    if let Some(rect) = parsed.default_percent_rect.as_ref() {
      validate_percent_rect(rect)?;
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let params = if context.params.is_null() {
      ManualCropParams::default()
    } else {
      serde_json::from_value::<ManualCropParams>(context.params.clone())?
    };

    let override_item = resolve_override(&params, &context.input_path);
    if let Some(item) = override_item {
      if item.skip {
        copy_original(&context.input_path, &context.output_path)?;
        return Ok(ProcessResult::success(
          "已跳过裁剪，原图已复制。",
          context.output_path.clone(),
        ));
      }
    }

    let (rgba, input_meta) = load_image_rgba(&context.input_path)?;
    let (source_width, source_height) = rgba.dimensions();

    let rect = resolve_crop_rect(&params, override_item, source_width, source_height)?;
    let cropped = crop_rgba_image(&rgba, rect)?;
    save_rgba_image(&context.output_path, &cropped)?;

    let (out_width, out_height) = cropped.dimensions();
    let mut result = ProcessResult::success("裁剪完成。", context.output_path.clone());
    result.input_metadata = Some(ImageMetadata {
      width: input_meta.width,
      height: input_meta.height,
      format: input_meta.format.clone(),
    });
    result.output_metadata = Some(ImageMetadata {
      width: out_width,
      height: out_height,
      format: Some("rgba8".to_string()),
    });

    Ok(result)
  }
}
