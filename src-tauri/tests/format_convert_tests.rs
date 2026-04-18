use art_tool_lib::core::processor::{ProcessContext, ProcessStatus, Processor};
use art_tool_lib::core::processors::format_convert::FormatConvertProcessor;
use image::{Rgba, RgbaImage};
use serde_json::json;
use tempfile::tempdir;

fn create_sample_png(path: &std::path::Path) {
  let mut image = RgbaImage::from_pixel(24, 18, Rgba([0, 0, 0, 0]));

  for y in 4..=13 {
    for x in 6..=20 {
      image.put_pixel(x, y, Rgba([220, 40, 55, 255]));
    }
  }

  image.save(path).expect("failed to save sample image");
}

#[test]
fn converts_png_to_jpg_successfully() {
  let tmp = tempdir().expect("create temp dir");
  let input_path = tmp.path().join("input.png");
  let output_path = tmp.path().join("converted.png");

  create_sample_png(&input_path);

  let processor = FormatConvertProcessor;
  let context = ProcessContext {
    processor_id: "format-convert".to_string(),
    input_path: input_path.clone(),
    output_path: output_path.clone(),
    params: json!({"targetFormat":"jpg"}),
  };

  let result = processor.process(&context).expect("process should succeed");

  assert_eq!(result.status, ProcessStatus::Success);
  let final_path = result.output_path.expect("output path should exist");
  assert!(final_path.exists());
  assert_eq!(
    final_path
      .extension()
      .and_then(|ext| ext.to_str())
      .expect("output extension should exist"),
    "jpg"
  );

  let decoded = image::ImageReader::open(&final_path)
    .expect("open converted image")
    .with_guessed_format()
    .expect("guess converted image format")
    .decode()
    .expect("decode converted image");

  assert_eq!(decoded.width(), 24);
  assert_eq!(decoded.height(), 18);
}

#[test]
fn rejects_unsupported_target_format() {
  let processor = FormatConvertProcessor;
  let validate = processor.validate(&json!({"targetFormat":"gif"}));

  assert!(validate.is_err());
}
