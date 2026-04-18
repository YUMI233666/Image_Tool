use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use serde::Deserialize;
use serde_json::Value;

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

#[derive(Default)]
pub struct FormatConvertProcessor;

impl Processor for FormatConvertProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "format-convert".to_string(),
      display_name: "图像格式转换".to_string(),
      enabled: true,
      notes: "支持 PNG/JPG/WEBP/BMP/TIFF 等常见格式互转。".to_string(),
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

    let source_image = ImageReader::open(&context.input_path)?
      .with_guessed_format()?
      .decode()?;
    let (source_width, source_height) = source_image.dimensions();

    let mut output_path = context.output_path.clone();
    output_path.set_extension(target_extension);
    if let Some(parent) = output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    let converted: DynamicImage = match target_format {
      ImageFormat::Jpeg => DynamicImage::ImageRgb8(source_image.to_rgb8()),
      _ => source_image.clone(),
    };

    converted.save_with_format(&output_path, target_format)?;

    let output_image = ImageReader::open(&output_path)?
      .with_guessed_format()?
      .decode()?;
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
