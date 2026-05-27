use art_tool_lib::core::processor::{ProcessContext, ProcessStatus, Processor};
use art_tool_lib::core::processors::manual_crop::ManualCropProcessor;
use image::{Rgba, RgbaImage};
use serde_json::json;
use tempfile::tempdir;

fn create_test_png(path: &std::path::Path) -> RgbaImage {
  let mut image = RgbaImage::from_pixel(12, 12, Rgba([0, 0, 0, 0]));
  for y in 2..6 {
    for x in 3..7 {
      image.put_pixel(x, y, Rgba([120, 200, 80, 180]));
    }
  }
  image.save(path).expect("save test png");
  image
}

#[test]
fn preserves_alpha_channel_when_cropping() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_test_png(&input_path);

  let processor = ManualCropProcessor;
  let context = ProcessContext {
    processor_id: "manual-crop".to_string(),
    input_path,
    output_path,
    params: json!({
      "applyMode": "absolute",
      "defaultRect": {
        "x": 1,
        "y": 1,
        "width": 6,
        "height": 6
      }
    }),
  };

  let result = processor.process(&context).expect("manual crop should succeed");
  assert_eq!(result.status, ProcessStatus::Success);

  let output = result.output_path.expect("output path should exist");
  let cropped = image::open(&output).expect("open cropped png").to_rgba8();
  assert_eq!(cropped.dimensions(), (6, 6));
  assert_eq!(cropped.get_pixel(2, 2).0[3], 180);
}

#[test]
fn rejects_out_of_bounds_absolute_crop() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_test_png(&input_path);

  let processor = ManualCropProcessor;
  let context = ProcessContext {
    processor_id: "manual-crop".to_string(),
    input_path,
    output_path,
    params: json!({
      "applyMode": "absolute",
      "defaultRect": {
        "x": 0,
        "y": 0,
        "width": 999,
        "height": 2
      }
    }),
  };

  let result = processor.process(&context);
  assert!(result.is_err());
}

#[test]
fn skip_copies_original_image() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  let source = create_test_png(&input_path);
  let input_key = input_path.to_string_lossy().to_string();

  let processor = ManualCropProcessor;
  let context = ProcessContext {
    processor_id: "manual-crop".to_string(),
    input_path,
    output_path,
    params: json!({
      "fileOverrides": {
        input_key: {
          "skip": true
        }
      }
    }),
  };

  let result = processor.process(&context).expect("skip should succeed");
  assert_eq!(result.status, ProcessStatus::Success);

  let output = result.output_path.expect("output path should exist");
  let copied = image::open(&output).expect("open copied png").to_rgba8();
  assert_eq!(copied.dimensions(), source.dimensions());
  assert_eq!(copied.get_pixel(3, 3), source.get_pixel(3, 3));
}
