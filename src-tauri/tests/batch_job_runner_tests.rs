use art_tool_lib::core::batch_job_runner::{
  run_batch_job,
  BatchJobRequest,
  WorkflowStepRequest,
};
use art_tool_lib::core::output_resolver::{RenameConfig, RenameMode};
use art_tool_lib::core::registry::ProcessorRegistry;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
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

fn create_test_jpg(path: &Path) {
  let mut image = RgbImage::from_pixel(80, 60, Rgb([0, 0, 0]));

  for y in 0..60 {
    for x in 0..80 {
      let r = ((x * 11 + y * 5) % 256) as u8;
      let g = ((x * 3 + y * 9) % 256) as u8;
      let b = ((x * 17 + y * 13) % 256) as u8;
      image.put_pixel(x, y, Rgb([r, g, b]));
    }
  }

  image
    .save(path)
    .expect("failed to save test jpg image");
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
    workflow_steps: vec![],
    rename_config: None,
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
    workflow_steps: vec![],
    rename_config: None,
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
    workflow_steps: vec![],
    rename_config: None,
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

#[test]
fn compress_batch_runs_successfully() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_jpg = input_dir.join("sample.jpg");
  create_test_jpg(&sample_jpg);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-4".to_string(),
    processor_id: "compress".to_string(),
    input_paths: vec![input_dir],
    output_dir: output_dir.clone(),
    params: json!({ "quality": 45, "mode": "lossy" }),
    workflow_steps: vec![],
    rename_config: None,
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("compress batch should finish");

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

#[test]
fn repair_batch_runs_successfully() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-5".to_string(),
    processor_id: "repair".to_string(),
    input_paths: vec![input_dir],
    output_dir: output_dir.clone(),
    params: json!({ "mode": "auto", "strength": 60 }),
    workflow_steps: vec![],
    rename_config: None,
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("repair batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 0);

  let output = report
    .items
    .iter()
    .find_map(|item| item.output_path.clone())
    .expect("output path should exist");

  assert!(output.ends_with(".png"));
  assert!(std::path::Path::new(&output).exists());
}

#[test]
fn repair_upscale_batch_increases_resolution() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-6".to_string(),
    processor_id: "repair".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: json!({ "mode": "upscale", "strength": 70, "upscaleFactor": 2 }),
    workflow_steps: vec![],
    rename_config: None,
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("repair upscale batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 0);

  let output = report
    .items
    .iter()
    .find_map(|item| item.output_path.clone())
    .expect("output path should exist");

  let output_image = image::open(std::path::Path::new(&output)).expect("open output image");
  assert_eq!(output_image.width(), 40);
  assert_eq!(output_image.height(), 40);
}

#[test]
fn resolution_transform_batch_supports_file_override() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-7".to_string(),
    processor_id: "resolution-transform".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: json!({
      "targetWidth": 120,
      "targetHeight": 120,
      "upscaleSharpness": 72,
      "fileOverrides": {
        sample_png.to_string_lossy().to_string(): {
          "targetWidth": 64,
          "targetHeight": 64
        }
      }
    }),
    workflow_steps: vec![],
    rename_config: None,
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("resolution transform batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 0);

  let output = report
    .items
    .iter()
    .find_map(|item| item.output_path.clone())
    .expect("output path should exist");

  let output_image = image::open(std::path::Path::new(&output)).expect("open output image");
  assert_eq!(output_image.width(), 64);
  assert_eq!(output_image.height(), 64);
}

#[test]
fn workflow_steps_with_rename_template_run_successfully() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-8".to_string(),
    processor_id: "trim-transparent".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: serde_json::Value::Null,
    workflow_steps: vec![
      WorkflowStepRequest {
        step_id: "step-1".to_string(),
        processor_id: "trim-transparent".to_string(),
        params: json!({ "alphaThreshold": 0, "padding": 0 }),
      },
      WorkflowStepRequest {
        step_id: "step-2".to_string(),
        processor_id: "format-convert".to_string(),
        params: json!({ "targetFormat": "jpg" }),
      },
    ],
    rename_config: Some(RenameConfig {
      enabled: true,
      mode: RenameMode::Template,
      custom_name: None,
      template: Some("asset_{index}".to_string()),
      start_index: 1,
      index_padding: 2,
    }),
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("workflow batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);
  assert_eq!(report.failed, 0);

  let item = report.items.first().expect("item should exist");
  assert_eq!(item.steps.len(), 2);

  let output_path = item
    .output_path
    .as_ref()
    .expect("output path should exist");
  assert!(output_path.ends_with("asset_01.jpg"));
}

#[test]
fn rename_custom_name_resolves_output_conflicts() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let first_png = input_dir.join("first.png");
  let second_png = input_dir.join("second.png");
  create_test_png(&first_png);
  create_test_png(&second_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-9".to_string(),
    processor_id: "trim-transparent".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: json!({ "alphaThreshold": 0, "padding": 0 }),
    workflow_steps: vec![],
    rename_config: Some(RenameConfig {
      enabled: true,
      mode: RenameMode::Custom,
      custom_name: Some("avatar".to_string()),
      template: None,
      start_index: 1,
      index_padding: 0,
    }),
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("rename batch should finish");

  assert_eq!(report.total, 2);
  assert_eq!(report.succeeded, 2);

  let output_names = report
    .items
    .iter()
    .filter_map(|item| item.output_path.as_ref())
    .map(|path| {
      std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
    })
    .collect::<Vec<_>>();

  assert!(output_names.contains(&"avatar.png".to_string()));
  assert!(output_names.contains(&"avatar_1.png".to_string()));
}

#[test]
fn rename_processor_runs_with_custom_name() {
  let temp = tempdir().expect("create temp dir");
  let input_dir = temp.path().join("input");
  let output_dir = temp.path().join("output");

  std::fs::create_dir_all(&input_dir).expect("create input dir");

  let sample_png = input_dir.join("sample.png");
  create_test_png(&sample_png);

  let registry = ProcessorRegistry::default_registry();
  let request = BatchJobRequest {
    job_id: "job-test-10".to_string(),
    processor_id: "rename".to_string(),
    input_paths: vec![input_dir],
    output_dir,
    params: serde_json::Value::Null,
    workflow_steps: vec![],
    rename_config: Some(RenameConfig {
      enabled: true,
      mode: RenameMode::Custom,
      custom_name: Some("item".to_string()),
      template: None,
      start_index: 1,
      index_padding: 0,
    }),
    max_concurrency: Some(1),
    include_subdirectories: true,
  };

  let report = run_batch_job(request, &registry, &|| false, &|_| {})
    .expect("rename processor batch should finish");

  assert_eq!(report.total, 1);
  assert_eq!(report.succeeded, 1);

  let output_path = report
    .items
    .iter()
    .find_map(|item| item.output_path.clone())
    .expect("output path should exist");

  assert!(output_path.ends_with("item.png"));
  assert!(std::path::Path::new(&output_path).exists());
}
