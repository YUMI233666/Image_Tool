use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use image::{imageops, DynamicImage, ImageReader, RgbaImage};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
  pub left: u32,
  pub top: u32,
  pub right: u32,
  pub bottom: u32,
}

impl BoundingBox {
  pub fn width(&self) -> u32 {
    self.right - self.left + 1
  }

  pub fn height(&self) -> u32 {
    self.bottom - self.top + 1
  }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrimTransparentParams {
  #[serde(default)]
  alpha_threshold: u8,
  #[serde(default)]
  padding: u32,
}

impl Default for TrimTransparentParams {
  fn default() -> Self {
    Self {
      alpha_threshold: 0,
      padding: 0,
    }
  }
}

#[derive(Default)]
pub struct TrimTransparentProcessor;

pub fn calculate_non_transparent_bbox(
  image: &RgbaImage,
  alpha_threshold: u8,
) -> Option<BoundingBox> {
  let (width, height) = image.dimensions();
  if width == 0 || height == 0 {
    return None;
  }

  let mut found = false;
  let mut left = width;
  let mut top = height;
  let mut right = 0;
  let mut bottom = 0;

  for (x, y, pixel) in image.enumerate_pixels() {
    if pixel.0[3] > alpha_threshold {
      found = true;
      left = left.min(x);
      top = top.min(y);
      right = right.max(x);
      bottom = bottom.max(y);
    }
  }

  if !found {
    return None;
  }

  Some(BoundingBox {
    left,
    top,
    right,
    bottom,
  })
}

impl Processor for TrimTransparentProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "trim-transparent".to_string(),
      display_name: "裁剪透明边缘".to_string(),
      enabled: true,
      notes: "裁剪 PNG 的透明像素边缘到非透明内容区域。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = if params.is_null() {
      TrimTransparentParams::default()
    } else {
      serde_json::from_value::<TrimTransparentParams>(params.clone())?
    };

    if parsed.padding > 10_000 {
      return Err(ProcessError::Validation(
        "padding 过大，请使用不超过 10000 的值。".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let params = if context.params.is_null() {
      TrimTransparentParams::default()
    } else {
      serde_json::from_value::<TrimTransparentParams>(context.params.clone())?
    };

    let source_image = ImageReader::open(&context.input_path)?
      .with_guessed_format()?
      .decode()?;

    let rgba = source_image.to_rgba8();
    let (source_width, source_height) = rgba.dimensions();

    let input_metadata = Some(ImageMetadata {
      width: source_width,
      height: source_height,
      format: Some("rgba8".to_string()),
    });

    let Some(bbox) = calculate_non_transparent_bbox(&rgba, params.alpha_threshold) else {
      let mut skipped = ProcessResult::skipped("图片完全透明，已跳过输出。");
      skipped.input_metadata = input_metadata;
      return Ok(skipped);
    };

    let left = bbox.left.saturating_sub(params.padding);
    let top = bbox.top.saturating_sub(params.padding);
    let right = bbox
      .right
      .saturating_add(params.padding)
      .min(source_width.saturating_sub(1));
    let bottom = bbox
      .bottom
      .saturating_add(params.padding)
      .min(source_height.saturating_sub(1));

    let crop_width = right - left + 1;
    let crop_height = bottom - top + 1;

    let cropped = imageops::crop_imm(&rgba, left, top, crop_width, crop_height).to_image();

    if let Some(parent) = context.output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    DynamicImage::ImageRgba8(cropped).save(&context.output_path)?;

    let mut success = ProcessResult::success("裁剪完成。", context.output_path.clone());
    success.input_metadata = input_metadata;
    success.output_metadata = Some(ImageMetadata {
      width: crop_width,
      height: crop_height,
      format: Some("png".to_string()),
    });

    Ok(success)
  }
}
