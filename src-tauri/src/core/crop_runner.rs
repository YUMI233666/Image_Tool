use crate::core::processor::{ImageMetadata, ProcessError};
use image::{imageops, DynamicImage, ImageFormat, ImageReader, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropRect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CropImageMeta {
  pub input: ImageMetadata,
  pub output: ImageMetadata,
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

pub fn load_image_rgba(path: &Path) -> Result<(RgbaImage, ImageMetadata), ProcessError> {
  let source_image = ImageReader::open(path)?.with_guessed_format()?.decode()?;
  let rgba = source_image.to_rgba8();
  let (width, height) = rgba.dimensions();
  let meta = ImageMetadata {
    width,
    height,
    format: Some("rgba8".to_string()),
  };
  Ok((rgba, meta))
}

fn validate_crop_rect(rect: CropRect, width: u32, height: u32) -> Result<(), ProcessError> {
  if rect.width == 0 || rect.height == 0 {
    return Err(ProcessError::Validation(
      "裁剪宽高必须大于 0。".to_string(),
    ));
  }

  if rect.x >= width || rect.y >= height {
    return Err(ProcessError::Validation(format!(
      "裁剪起点超出图片尺寸（图片: {width}x{height}）。",
    )));
  }

  if rect.x.saturating_add(rect.width) > width
    || rect.y.saturating_add(rect.height) > height
  {
    return Err(ProcessError::Validation(format!(
      "裁剪范围超出图片尺寸（图片: {width}x{height}，裁剪: x={}, y={}, w={}, h={}）。",
      rect.x, rect.y, rect.width, rect.height
    )));
  }

  Ok(())
}

pub fn crop_rgba_image(
  source: &RgbaImage,
  rect: CropRect,
) -> Result<RgbaImage, ProcessError> {
  let (width, height) = source.dimensions();
  validate_crop_rect(rect, width, height)?;
  Ok(imageops::crop_imm(source, rect.x, rect.y, rect.width, rect.height).to_image())
}

pub fn save_rgba_image(path: &Path, image: &RgbaImage) -> Result<(), ProcessError> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }

  let output = DynamicImage::ImageRgba8(image.clone());
  if let Some(format) = parse_output_format(path) {
    match format {
      ImageFormat::Jpeg => {
        let rgb = output.to_rgb8();
        DynamicImage::ImageRgb8(rgb).save_with_format(path, format)?;
      }
      _ => output.save_with_format(path, format)?,
    }
  } else {
    output.save(path)?;
  }

  Ok(())
}

pub fn crop_image_file(
  input_path: &Path,
  output_path: &Path,
  rect: CropRect,
) -> Result<CropImageMeta, ProcessError> {
  let (rgba, input_meta) = load_image_rgba(input_path)?;
  let cropped = crop_rgba_image(&rgba, rect)?;
  save_rgba_image(output_path, &cropped)?;
  let (width, height) = cropped.dimensions();
  let output_meta = ImageMetadata {
    width,
    height,
    format: Some("rgba8".to_string()),
  };
  Ok(CropImageMeta {
    input: input_meta,
    output: output_meta,
  })
}
