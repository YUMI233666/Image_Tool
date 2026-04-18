use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{
  CompressionType as PngCompressionType,
  FilterType as PngFilterType,
  PngEncoder,
};
use image::{
  ColorType,
  GenericImageView,
  ImageEncoder,
  ImageFormat,
  ImageReader,
  RgbaImage,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompressParams {
  #[serde(default = "default_quality")]
  quality: u8,
  #[serde(default = "default_mode")]
  mode: String,
}

fn default_quality() -> u8 {
  80
}

fn default_mode() -> String {
  "balanced".to_string()
}

fn normalize_mode(raw: &str) -> Option<&'static str> {
  match raw.trim().to_ascii_lowercase().as_str() {
    "lossy" => Some("lossy"),
    "lossless" => Some("lossless"),
    "balanced" => Some("balanced"),
    _ => None,
  }
}

fn to_human_size(size: u64) -> String {
  const KB: f64 = 1024.0;
  const MB: f64 = KB * 1024.0;
  const GB: f64 = MB * 1024.0;

  let value = size as f64;
  if value >= GB {
    format!("{:.2} GB", value / GB)
  } else if value >= MB {
    format!("{:.2} MB", value / MB)
  } else if value >= KB {
    format!("{:.2} KB", value / KB)
  } else {
    format!("{} B", size)
  }
}

fn extension_of(path: &Path) -> Option<String> {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.to_ascii_lowercase())
}

fn quantize_component(value: u8, bits: u8) -> u8 {
  if bits >= 8 {
    return value;
  }

  let max_level = (1u16 << bits) - 1;
  let quantized = ((value as u16 * max_level) + 127) / 255;
  (((quantized * 255) + (max_level / 2)) / max_level) as u8
}

fn png_quantization_bits(mode: &str, quality: u8) -> (u8, u8) {
  let q = quality.clamp(1, 100);
  match mode {
    "lossless" => (8, 8),
    "balanced" => {
      if q >= 90 {
        (8, 8)
      } else if q >= 75 {
        (7, 8)
      } else if q >= 60 {
        (6, 7)
      } else if q >= 45 {
        (6, 6)
      } else if q >= 30 {
        (5, 6)
      } else {
        (5, 5)
      }
    }
    "lossy" => {
      if q >= 85 {
        (7, 7)
      } else if q >= 70 {
        (6, 7)
      } else if q >= 55 {
        (6, 6)
      } else if q >= 40 {
        (5, 6)
      } else if q >= 25 {
        (5, 5)
      } else {
        (4, 4)
      }
    }
    _ => (8, 8),
  }
}

fn posterize_rgba_for_png(rgba: &mut RgbaImage, color_bits: u8, alpha_bits: u8) {
  if color_bits >= 8 && alpha_bits >= 8 {
    return;
  }

  for pixel in rgba.pixels_mut() {
    pixel.0[0] = quantize_component(pixel.0[0], color_bits);
    pixel.0[1] = quantize_component(pixel.0[1], color_bits);
    pixel.0[2] = quantize_component(pixel.0[2], color_bits);
    pixel.0[3] = quantize_component(pixel.0[3], alpha_bits);
  }
}

fn is_fully_opaque(rgba: &RgbaImage) -> bool {
  rgba.pixels().all(|pixel| pixel.0[3] == 255)
}

fn rgba_to_rgb_bytes(rgba: &RgbaImage) -> Vec<u8> {
  let mut bytes = Vec::with_capacity((rgba.width() * rgba.height() * 3) as usize);
  for pixel in rgba.pixels() {
    bytes.push(pixel.0[0]);
    bytes.push(pixel.0[1]);
    bytes.push(pixel.0[2]);
  }
  bytes
}

fn encode_jpeg(
  image: &image::DynamicImage,
  output_path: &Path,
  quality: u8,
) -> Result<(), ProcessError> {
  let rgb = image.to_rgb8();
  let (width, height) = rgb.dimensions();
  let writer = BufWriter::new(File::create(output_path)?);
  let mut encoder = JpegEncoder::new_with_quality(writer, quality);
  encoder.encode(&rgb, width, height, ColorType::Rgb8.into())?;
  Ok(())
}

fn encode_png(
  image: &image::DynamicImage,
  output_path: &Path,
  mode: &str,
  quality: u8,
) -> Result<(), ProcessError> {
  let mut rgba = image.to_rgba8();
  let (width, height) = rgba.dimensions();

  let (color_bits, alpha_bits) = png_quantization_bits(mode, quality);
  if mode != "lossless" {
    posterize_rgba_for_png(&mut rgba, color_bits, alpha_bits);
  }

  let compression = match mode {
    "lossless" => PngCompressionType::Best,
    "lossy" => PngCompressionType::Best,
    _ => {
      if quality >= 85 {
        PngCompressionType::Best
      } else if quality <= 40 {
        PngCompressionType::Default
      } else {
        PngCompressionType::Default
      }
    }
  };

  let writer = BufWriter::new(File::create(output_path)?);
  let encoder = PngEncoder::new_with_quality(writer, compression, PngFilterType::Adaptive);

  if is_fully_opaque(&rgba) {
    let rgb_bytes = rgba_to_rgb_bytes(&rgba);
    encoder.write_image(&rgb_bytes, width, height, ColorType::Rgb8.into())?;
  } else {
    encoder.write_image(rgba.as_raw(), width, height, ColorType::Rgba8.into())?;
  }

  Ok(())
}

fn encode_webp(image: &image::DynamicImage, output_path: &Path) -> Result<(), ProcessError> {
  image.save_with_format(output_path, ImageFormat::WebP)?;
  Ok(())
}

#[derive(Default)]
pub struct CompressProcessor;

impl Processor for CompressProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "compress".to_string(),
      display_name: "图像压缩".to_string(),
      enabled: true,
      notes: "支持 JPG/PNG/WEBP 压缩，含 lossy/lossless/balanced 模式。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = serde_json::from_value::<CompressParams>(params.clone())?;

    if !(1..=100).contains(&parsed.quality) {
      return Err(ProcessError::Validation(
        "quality 必须在 1 到 100 之间。".to_string(),
      ));
    }

    if normalize_mode(&parsed.mode).is_none() {
      return Err(ProcessError::Validation(
        "mode 不受支持，请使用 lossy/lossless/balanced。".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let parsed = if context.params.is_null() {
      CompressParams {
        quality: default_quality(),
        mode: default_mode(),
      }
    } else {
      serde_json::from_value::<CompressParams>(context.params.clone())?
    };

    let mode = normalize_mode(&parsed.mode).ok_or_else(|| {
      ProcessError::Validation("mode 不受支持，请使用 lossy/lossless/balanced。".to_string())
    })?;

    let input_size = fs::metadata(&context.input_path)?.len();
    let source_image = ImageReader::open(&context.input_path)?
      .with_guessed_format()?
      .decode()?;
    let (source_width, source_height) = source_image.dimensions();

    let output_path = context.output_path.clone();
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    let extension = extension_of(&output_path).unwrap_or_default();
    let mut mode_note = String::new();

    match extension.as_str() {
      "jpg" | "jpeg" => {
        let quality = if mode == "lossless" {
          mode_note = "（JPEG 不支持真正无损，已自动使用质量 100）".to_string();
          100
        } else {
          parsed.quality
        };
        encode_jpeg(&source_image, &output_path, quality)?;
      }
      "png" => {
        encode_png(&source_image, &output_path, mode, parsed.quality)?;
      }
      "webp" => {
        if mode == "lossy" {
          mode_note = "（当前 WebP 走默认编码，未细分 lossy 质量参数）".to_string();
        }
        encode_webp(&source_image, &output_path)?;
      }
      "bmp" | "tiff" => {
        let mut skipped = ProcessResult::skipped(
          "当前版本暂不支持 BMP/TIFF 压缩，建议先转换为 JPG/PNG/WEBP。",
        );
        skipped.input_metadata = Some(ImageMetadata {
          width: source_width,
          height: source_height,
          format: Some(extension),
        });
        return Ok(skipped);
      }
      _ => {
        let mut skipped = ProcessResult::skipped("不支持的输出格式，已跳过压缩。".to_string());
        skipped.input_metadata = Some(ImageMetadata {
          width: source_width,
          height: source_height,
          format: Some(extension),
        });
        return Ok(skipped);
      }
    }

    let output_size = fs::metadata(&output_path)?.len();
    if output_size >= input_size {
      fs::copy(&context.input_path, &output_path)?;
      let mut skipped = ProcessResult::skipped("压缩后体积未下降，已保留原文件。");
      skipped.output_path = Some(output_path.clone());
      skipped.input_metadata = Some(ImageMetadata {
        width: source_width,
        height: source_height,
        format: extension_of(&context.input_path),
      });
      skipped.output_metadata = Some(ImageMetadata {
        width: source_width,
        height: source_height,
        format: extension_of(&output_path),
      });
      return Ok(skipped);
    }

    let ratio = if input_size == 0 {
      0.0
    } else {
      (1.0 - (output_size as f64 / input_size as f64)) * 100.0
    };
    let message = format!(
      "压缩完成: {} -> {} ({:+.2}%){}",
      to_human_size(input_size),
      to_human_size(output_size),
      ratio,
      mode_note
    );

    let mut result = ProcessResult::success(message, output_path.clone());
    result.input_metadata = Some(ImageMetadata {
      width: source_width,
      height: source_height,
      format: extension_of(&context.input_path),
    });
    result.output_metadata = Some(ImageMetadata {
      width: source_width,
      height: source_height,
      format: extension_of(&output_path),
    });

    Ok(result)
  }
}
