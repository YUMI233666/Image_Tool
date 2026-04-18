use art_tool_lib::core::batch_job_runner::{run_batch_job, BatchJobRequest};
use art_tool_lib::core::registry::ProcessorRegistry;
use image::{Rgba, RgbaImage};
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;

fn create_test_png(path: &Path) {
  let mut image = RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 0]));

  for y in 6..=14 {
    for x in 5..=16 {
      image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
    }
  }

  image.save(path).expect("failed to save test image");
}

#[test]
fn continues_after_single_file_failure() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let valid_png = input_dir.join("valid.png");
  let invalid_png = input_dir.join("invalid.png");

  create_test_png(&valid_png);
  std::fs::write(&invalid_png, b"not-a-real-png").expect("write invalid png");

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-1".to_string(),
    processor_id: "trim-transparent".to_string(),
    input_paths: vec![input_dir.clone()],
    output_dir: output_dir.clone(),
    params: json!({ "alphaThreshold": 0, "padding": 0 }),
    max_concurrency: Some(2),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("batch runner should finish");

  assert_eq!(report.total, 2);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 1);
  assert_eq!(report.items.len(), 2);
  assert!(output_dir.exists());
}

#[test]
fn cancellation_marks_remaining_items() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-2".to_string(),
    processor_id: "trim-transparent".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: json!({ "alphaThreshold": 0, "padding": 0 }),
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| true, &|_| {})
    .expect("batch runner should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.cancelled, 1);
  assert_eq!(report.succeeded, 0);
}

#[test]
fn format_convert_generates_target_extension() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-3".to_string(),
    processor_id: "format-convert".to_string(),
    input_paths: vec![input_dir],
    output_dir: output_dir.clone(),
    params: json!({ "targetFormat": "jpg" }),
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("format convert batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 0);

  let output = report
    .items
    .iter()
    .find_map(|item| item.output_path.clone())
    .expect("output path should exist");

  assert!(output.ends_with(".jpg"));
  assert!(std::path::Path::new(&output).exists());
}
