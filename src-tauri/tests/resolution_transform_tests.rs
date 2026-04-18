use art_tool_lib::core::processor::{ProcessContext, ProcessStatus, Processor};
use art_tool_lib::core::processors::resolution_transform::ResolutionTransformProcessor;
use image::{GenericImageView, Rgb, RgbImage, Rgba, RgbaImage};
use serde_json::json;
use tempfile::tempdir;

fn create_subject_png(path: &std::path::Path) {
  let mut image = RgbaImage::from_pixel(120, 120, Rgba([0, 0, 0, 0]));

  for y in 40..80 {
    for x in 20..100 {
      image.put_pixel(x, y, Rgba([245, 180, 80, 255]));
    }
  }

  image.save(path).expect("save subject png");
}

fn create_test_jpg(path: &std::path::Path) {
  let mut image = RgbImage::new(120, 80);

  for y in 0..80 {
    for x in 0..120 {
      let r = ((x * 9 + y * 4 + 30) % 256) as u8;
      let g = ((x * 5 + y * 11 + 90) % 256) as u8;
      let b = ((x * 13 + y * 7 + 55) % 256) as u8;
      image.put_pixel(x, y, Rgb([r, g, b]));
    }
  }

  image.save(path).expect("save test jpg");
}

#[test]
fn rejects_invalid_target_resolution() {
  let processor = ResolutionTransformProcessor;

  let validation = processor.validate(&json!({
    "targetWidth": 0,
    "targetHeight": 1080,
    "upscaleSharpness": 70
  }));

  assert!(validation.is_err());
}

#[test]
fn rejects_invalid_upscale_sharpness() {
  let processor = ResolutionTransformProcessor;

  let validation = processor.validate(&json!({
    "targetWidth": 1920,
    "targetHeight": 1080,
    "upscaleSharpness": 0
  }));

  assert!(validation.is_err());
}

#[test]
fn png_ratio_mismatch_is_center_padded_with_transparency() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_subject_png(&input_path);

  let processor = ResolutionTransformProcessor;
  let context = ProcessContext {
    processor_id: "resolution-transform".to_string(),
    input_path,
    output_path,
    params: json!({
      "targetWidth": 200,
      "targetHeight": 200,
      "upscaleSharpness": 72
    }),
  };

  let result = processor
    .process(&context)
    .expect("resolution transform png should succeed");
  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.message.contains("透明居中填充"));

  let output = result.output_path.expect("output path should exist");
  let transformed = image::open(&output)
    .expect("open transformed png")
    .to_rgba8();

  assert_eq!(transformed.dimensions(), (200, 200));
  assert_eq!(transformed.get_pixel(100, 8).0[3], 0);
  assert_eq!(transformed.get_pixel(100, 192).0[3], 0);
  assert!(transformed.get_pixel(100, 100).0[3] > 0);
}

#[test]
fn jpg_ratio_mismatch_keeps_aspect_ratio() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.jpg");
  let output_path = temp.path().join("output.jpg");

  create_test_jpg(&input_path);

  let processor = ResolutionTransformProcessor;
  let context = ProcessContext {
    processor_id: "resolution-transform".to_string(),
    input_path,
    output_path,
    params: json!({
      "targetWidth": 200,
      "targetHeight": 200,
      "upscaleSharpness": 75
    }),
  };

  let result = processor
    .process(&context)
    .expect("resolution transform jpg should succeed");
  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.message.contains("按原比例适配"));

  let output = result.output_path.expect("output path should exist");
  let transformed = image::open(&output).expect("open transformed jpg");
  let (width, height) = transformed.dimensions();

  assert!(width == 200 || height == 200);
  assert!(width < 200 || height < 200);

  let ratio = width as f32 / height as f32;
  assert!((ratio - 1.5).abs() < 0.06);
}

#[test]
fn uses_per_file_override_target() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("override-target.png");
  let output_path = temp.path().join("override-target-out.png");

  create_subject_png(&input_path);

  let input_key = input_path.to_string_lossy().to_string();

  let processor = ResolutionTransformProcessor;
  let context = ProcessContext {
    processor_id: "resolution-transform".to_string(),
    input_path,
    output_path,
    params: json!({
      "targetWidth": 300,
      "targetHeight": 300,
      "upscaleSharpness": 70,
      "fileOverrides": {
        input_key: {
          "targetWidth": 64,
          "targetHeight": 64
        }
      }
    }),
  };

  let result = processor
    .process(&context)
    .expect("resolution transform with override should succeed");
  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.message.contains("已应用单文件目标分辨率"));

  let output = result.output_path.expect("output path should exist");
  let transformed = image::open(&output)
    .expect("open transformed output")
    .to_rgba8();

  assert_eq!(transformed.dimensions(), (64, 64));
}
