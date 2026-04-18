use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use image::{
  DynamicImage,
  GenericImageView,
  ImageFormat,
  ImageReader,
  Rgb,
  RgbImage,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatConvertParams {
  target_format: String,
}

fn parse_target_format(raw: &str) -> Option<(&'static str, ImageFormat)> {
  let normalized = raw.trim().to_ascii_lowercase();
  match normalized.as_str() {
    "png" => Some(("png", ImageFormat::Png)),
    "jpg" | "jpeg" => Some(("jpg", ImageFormat::Jpeg)),
    "webp" => Some(("webp", ImageFormat::WebP)),
    "bmp" => Some(("bmp", ImageFormat::Bmp)),
    "tiff" => Some(("tiff", ImageFormat::Tiff)),
    _ => None,
  }
}

fn parse_input_format_by_extension(path: &Path) -> Option<ImageFormat> {
  let extension = path.extension()?.to_str()?.trim().to_ascii_lowercase();

  match extension.as_str() {
    "png" => Some(ImageFormat::Png),
    "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
    "webp" => Some(ImageFormat::WebP),
    "bmp" => Some(ImageFormat::Bmp),
    "tiff" | "tif" => Some(ImageFormat::Tiff),
    _ => None,
  }
}

fn decode_with_fallback(path: &Path) -> Result<DynamicImage, ProcessError> {
  let reader = ImageReader::open(path)?;
  let guessed = reader.with_guessed_format()?;

  match guessed.decode() {
    Ok(image) => Ok(image),
    Err(primary_error) => {
      let Some(fallback_format) = parse_input_format_by_extension(path) else {
        return Err(ProcessError::Image(primary_error));
      };

      let mut fallback_reader = ImageReader::open(path)?;
      fallback_reader.set_format(fallback_format);
      let fallback = fallback_reader.decode();

      fallback.map_err(|_| ProcessError::Image(primary_error))
    }
  }
}

fn flatten_to_rgb_with_white_background(source: &DynamicImage) -> DynamicImage {
  let rgba = source.to_rgba8();
  let (width, height) = rgba.dimensions();
  let mut rgb = RgbImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let pixel = rgba.get_pixel(x, y).0;
      let alpha = pixel[3] as f32 / 255.0;

      let compose = |channel: u8| {
        let base = channel as f32 * alpha;
        let background = 255.0 * (1.0 - alpha);
        (base + background).round().clamp(0.0, 255.0) as u8
      };

      rgb.put_pixel(
        x,
        y,
        Rgb([compose(pixel[0]), compose(pixel[1]), compose(pixel[2])]),
      );
    }
  }

  DynamicImage::ImageRgb8(rgb)
}

#[derive(Default)]
pub struct FormatConvertProcessor;

impl Processor for FormatConvertProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "format-convert".to_string(),
      display_name: "图像格式转换".to_string(),
      enabled: true,
      notes: "支持 PNG/JPG/WEBP 格式互转。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = serde_json::from_value::<FormatConvertParams>(params.clone())?;

    if parse_target_format(&parsed.target_format).is_none() {
      return Err(ProcessError::Validation(
        "targetFormat 不受支持，请使用 png/jpg/webp/bmp/tiff".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let params = serde_json::from_value::<FormatConvertParams>(context.params.clone())?;
    let (target_extension, target_format) = parse_target_format(&params.target_format)
      .ok_or_else(|| {
      ProcessError::Validation("targetFormat 不受支持，请使用 png/jpg/webp/bmp/tiff".to_string())
    })?;

    let source_image = decode_with_fallback(&context.input_path)?;
    let (source_width, source_height) = source_image.dimensions();

    let mut output_path = context.output_path.clone();
    output_path.set_extension(target_extension);
    if let Some(parent) = output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    let converted: DynamicImage = match target_format {
      ImageFormat::Jpeg => flatten_to_rgb_with_white_background(&source_image),
      _ => source_image.clone(),
    };

    converted.save_with_format(&output_path, target_format)?;

    let output_image = decode_with_fallback(&output_path)?;
    let (output_width, output_height) = output_image.dimensions();

    let mut result = ProcessResult::success("格式转换完成。", output_path);
    result.input_metadata = Some(ImageMetadata {
      width: source_width,
      height: source_height,
      format: Some("source".to_string()),
    });
    result.output_metadata = Some(ImageMetadata {
      width: output_width,
      height: output_height,
      format: Some(target_extension.to_string()),
    });

    Ok(result)
  }
}
