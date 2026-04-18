use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use crate::core::processors::trim_transparent::calculate_non_transparent_bbox;
use image::{
  imageops,
  imageops::FilterType,
  DynamicImage,
  GenericImageView,
  ImageFormat,
  ImageReader,
  Rgba,
  RgbaImage,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

const MAX_TARGET_EDGE: u32 = 16_384;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolutionTarget {
  target_width: u32,
  target_height: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolutionTransformParams {
  #[serde(default = "default_target_width")]
  target_width: u32,
  #[serde(default = "default_target_height")]
  target_height: u32,
  #[serde(default = "default_upscale_sharpness")]
  upscale_sharpness: u8,
  #[serde(default)]
  file_overrides: HashMap<String, ResolutionTarget>,
}

fn default_target_width() -> u32 {
  1920
}

fn default_target_height() -> u32 {
  1080
}

fn default_upscale_sharpness() -> u8 {
  70
}

impl Default for ResolutionTransformParams {
  fn default() -> Self {
    Self {
      target_width: default_target_width(),
      target_height: default_target_height(),
      upscale_sharpness: default_upscale_sharpness(),
      file_overrides: HashMap::new(),
    }
  }
}

fn normalize_path_key(raw: &str) -> String {
  raw.replace('\\', "/").to_ascii_lowercase()
}

fn extension_of(path: &Path) -> Option<String> {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.to_ascii_lowercase())
}

fn parse_output_format(path: &Path) -> Option<ImageFormat> {
  match extension_of(path).as_deref() {
    Some("png") => Some(ImageFormat::Png),
    Some("jpg") | Some("jpeg") => Some(ImageFormat::Jpeg),
    Some("webp") => Some(ImageFormat::WebP),
    Some("bmp") => Some(ImageFormat::Bmp),
    Some("tiff") | Some("tif") => Some(ImageFormat::Tiff),
    _ => None,
  }
}

fn blend_channel(base: u8, overlay: u8, weight: f32) -> u8 {
  let w = weight.clamp(0.0, 1.0);
  let value = (base as f32 * (1.0 - w)) + (overlay as f32 * w);
  value.round().clamp(0.0, 255.0) as u8
}

fn blend_images(base: &RgbaImage, overlay: &RgbaImage, weight: f32) -> RgbaImage {
  let (width, height) = base.dimensions();
  let mut output = base.clone();

  for y in 0..height {
    for x in 0..width {
      let b = base.get_pixel(x, y);
      let o = overlay.get_pixel(x, y);
      output.put_pixel(
        x,
        y,
        Rgba([
          blend_channel(b.0[0], o.0[0], weight),
          blend_channel(b.0[1], o.0[1], weight),
          blend_channel(b.0[2], o.0[2], weight),
          b.0[3],
        ]),
      );
    }
  }

  output
}

fn sharpen_channel(base: u8, blurred: u8, amount: f32) -> u8 {
  let detail = (base as f32 - blurred as f32).clamp(-50.0, 50.0);
  let value = base as f32 + detail * amount;
  value.round().clamp(0.0, 255.0) as u8
}

fn fit_inside_target(
  source_width: u32,
  source_height: u32,
  target_width: u32,
  target_height: u32,
) -> (u32, u32, f32) {
  let scale = (target_width as f64 / source_width as f64)
    .min(target_height as f64 / source_height as f64);

  let width = (source_width as f64 * scale)
    .round()
    .clamp(1.0, target_width as f64) as u32;
  let height = (source_height as f64 * scale)
    .round()
    .clamp(1.0, target_height as f64) as u32;

  (width, height, scale as f32)
}

fn resize_with_upscale_enhancement(
  source: &RgbaImage,
  target_width: u32,
  target_height: u32,
  sharpness: u8,
) -> (RgbaImage, f32) {
  let (resized_width, resized_height, scale) = fit_inside_target(
    source.width(),
    source.height(),
    target_width,
    target_height,
  );

  if scale <= 1.0 {
    let downscaled = imageops::resize(source, resized_width, resized_height, FilterType::Lanczos3);
    return (downscaled, scale);
  }

  let sharpness_norm = sharpness as f32 / 100.0;
  let lanczos = imageops::resize(source, resized_width, resized_height, FilterType::Lanczos3);
  let catmull = imageops::resize(source, resized_width, resized_height, FilterType::CatmullRom);

  let interpolation_blend = (0.18 + 0.52 * sharpness_norm).clamp(0.0, 0.82);
  let mixed = blend_images(&lanczos, &catmull, interpolation_blend);

  let blurred = imageops::blur(&mixed, 0.55 + 0.75 * (1.0 - sharpness_norm));
  let sharpen_amount = 0.32 + 1.36 * sharpness_norm;

  let mut enhanced = mixed.clone();
  for y in 0..resized_height {
    for x in 0..resized_width {
      let base = mixed.get_pixel(x, y);
      let soft = blurred.get_pixel(x, y);
      enhanced.put_pixel(
        x,
        y,
        Rgba([
          sharpen_channel(base.0[0], soft.0[0], sharpen_amount),
          sharpen_channel(base.0[1], soft.0[1], sharpen_amount),
          sharpen_channel(base.0[2], soft.0[2], sharpen_amount),
          base.0[3],
        ]),
      );
    }
  }

  (enhanced, scale)
}

fn center_on_transparent_canvas(content: &RgbaImage, width: u32, height: u32) -> RgbaImage {
  let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
  let offset_x = (width.saturating_sub(content.width())) / 2;
  let offset_y = (height.saturating_sub(content.height())) / 2;

  for y in 0..content.height() {
    for x in 0..content.width() {
      let pixel = *content.get_pixel(x, y);
      canvas.put_pixel(offset_x + x, offset_y + y, pixel);
    }
  }

  canvas
}

fn save_output_image(path: &Path, image: &RgbaImage) -> Result<(), ProcessError> {
  let format = parse_output_format(path);

  let output = match format {
    Some(ImageFormat::Jpeg) => {
      DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(image.clone()).to_rgb8())
    }
    _ => DynamicImage::ImageRgba8(image.clone()),
  };

  if let Some(output_format) = format {
    output.save_with_format(path, output_format)?;
  } else {
    output.save(path)?;
  }

  Ok(())
}

fn validate_resolution_target(
  width: u32,
  height: u32,
  prefix: &str,
) -> Result<(), ProcessError> {
  if !(1..=MAX_TARGET_EDGE).contains(&width) {
    return Err(ProcessError::Validation(format!(
      "{prefix}targetWidth 必须在 1 到 {MAX_TARGET_EDGE} 之间。"
    )));
  }

  if !(1..=MAX_TARGET_EDGE).contains(&height) {
    return Err(ProcessError::Validation(format!(
      "{prefix}targetHeight 必须在 1 到 {MAX_TARGET_EDGE} 之间。"
    )));
  }

  Ok(())
}

fn resolve_target_for_input(
  params: &ResolutionTransformParams,
  input_path: &Path,
) -> (u32, u32, bool) {
  let default_target = (params.target_width, params.target_height, false);
  if params.file_overrides.is_empty() {
    return default_target;
  }

  let input_raw = input_path.to_string_lossy().to_string();
  if let Some(target) = params.file_overrides.get(&input_raw) {
    return (target.target_width, target.target_height, true);
  }

  let input_key = normalize_path_key(&input_raw);

  for (path_key, target) in &params.file_overrides {
    if normalize_path_key(path_key) == input_key {
      return (target.target_width, target.target_height, true);
    }
  }

  default_target
}

fn parse_ratio(width: u32, height: u32) -> f32 {
  if height == 0 {
    return 0.0;
  }

  width as f32 / height as f32
}

#[derive(Default)]
pub struct ResolutionTransformProcessor;

impl Processor for ResolutionTransformProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "resolution-transform".to_string(),
      display_name: "变换分辨率".to_string(),
      enabled: true,
      notes: "支持按目标分辨率缩放/超分，PNG 可自动透明居中填充；支持批量默认值和单文件覆盖。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = if params.is_null() {
      ResolutionTransformParams::default()
    } else {
      serde_json::from_value::<ResolutionTransformParams>(params.clone())?
    };

    validate_resolution_target(parsed.target_width, parsed.target_height, "")?;

    if !(1..=100).contains(&parsed.upscale_sharpness) {
      return Err(ProcessError::Validation(
        "upscaleSharpness 必须在 1 到 100 之间。".to_string(),
      ));
    }

    for (path, target) in &parsed.file_overrides {
      validate_resolution_target(
        target.target_width,
        target.target_height,
        &format!("fileOverrides[{path}]."),
      )?;
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let params = if context.params.is_null() {
      ResolutionTransformParams::default()
    } else {
      serde_json::from_value::<ResolutionTransformParams>(context.params.clone())?
    };

    let source_image = ImageReader::open(&context.input_path)?
      .with_guessed_format()?
      .decode()?;
    let source_rgba = source_image.to_rgba8();
    let (source_width, source_height) = source_image.dimensions();

    let (target_width, target_height, is_override) =
      resolve_target_for_input(&params, &context.input_path);

    let output_ext = extension_of(&context.output_path).unwrap_or_default();
    let output_is_png = output_ext == "png";
    let output_is_jpg_or_webp = output_ext == "jpg" || output_ext == "jpeg" || output_ext == "webp";

    let (content_for_resize, content_ratio_width, content_ratio_height, transparent_only) =
      if output_is_png {
        if let Some(bbox) = calculate_non_transparent_bbox(&source_rgba, 0) {
          (
            imageops::crop_imm(
              &source_rgba,
              bbox.left,
              bbox.top,
              bbox.width(),
              bbox.height(),
            )
            .to_image(),
            bbox.width(),
            bbox.height(),
            false,
          )
        } else {
          (RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])), 1, 1, true)
        }
      } else {
        (source_rgba.clone(), source_width, source_height, false)
      };

    let source_ratio = parse_ratio(content_ratio_width, content_ratio_height);
    let target_ratio = parse_ratio(target_width, target_height);
    let ratio_mismatch = (source_ratio - target_ratio).abs() > 0.0001;

    let (final_image, operation_label, padded) = if transparent_only && output_is_png {
      (
        RgbaImage::from_pixel(target_width, target_height, Rgba([0, 0, 0, 0])),
        "透明占位".to_string(),
        true,
      )
    } else {
      let (resized, scale) = resize_with_upscale_enhancement(
        &content_for_resize,
        target_width,
        target_height,
        params.upscale_sharpness,
      );

      let operation = if scale > 1.001 {
        "超分放大"
      } else if scale < 0.999 {
        "缩放压缩"
      } else {
        "尺寸保持"
      }
      .to_string();

      if output_is_png {
        if resized.width() == target_width && resized.height() == target_height {
          (resized, operation, false)
        } else {
          (
            center_on_transparent_canvas(&resized, target_width, target_height),
            operation,
            true,
          )
        }
      } else {
        (resized, operation, false)
      }
    };

    if let Some(parent) = context.output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    save_output_image(&context.output_path, &final_image)?;

    let output_width = final_image.width();
    let output_height = final_image.height();

    let override_note = if is_override {
      "（已应用单文件目标分辨率）"
    } else {
      ""
    };

    let message = if output_is_png {
      if padded {
        format!(
          "分辨率变换完成: {}x{} -> {}x{}，目标 {}x{}，{}，已透明居中填充{}。",
          source_width,
          source_height,
          output_width,
          output_height,
          target_width,
          target_height,
          operation_label,
          override_note,
        )
      } else {
        format!(
          "分辨率变换完成: {}x{} -> {}x{}，目标 {}x{}，{}{}。",
          source_width,
          source_height,
          output_width,
          output_height,
          target_width,
          target_height,
          operation_label,
          override_note,
        )
      }
    } else if ratio_mismatch && output_is_jpg_or_webp {
      format!(
        "分辨率变换完成: {}x{} -> {}x{}，目标 {}x{}，{}。目标比例与原图不一致，已按原比例适配{}。",
        source_width,
        source_height,
        output_width,
        output_height,
        target_width,
        target_height,
        operation_label,
        override_note,
      )
    } else {
      format!(
        "分辨率变换完成: {}x{} -> {}x{}，目标 {}x{}，{}{}。",
        source_width,
        source_height,
        output_width,
        output_height,
        target_width,
        target_height,
        operation_label,
        override_note,
      )
    };

    let mut result = ProcessResult::success(message, context.output_path.clone());
    result.input_metadata = Some(ImageMetadata {
      width: source_width,
      height: source_height,
      format: extension_of(&context.input_path),
    });
    result.output_metadata = Some(ImageMetadata {
      width: output_width,
      height: output_height,
      format: extension_of(&context.output_path),
    });

    Ok(result)
  }
}
