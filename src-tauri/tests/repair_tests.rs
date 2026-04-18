use art_tool_lib::core::processor::{ProcessContext, ProcessStatus, Processor};
use art_tool_lib::core::processors::repair::RepairProcessor;
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use serde_json::json;
use tempfile::tempdir;

fn create_noisy_png(path: &std::path::Path) {
  let width = 220;
  let height = 160;
  let mut image = RgbaImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let r = ((x * 17 + y * 29 + 13) % 256) as u8;
      let g = ((x * 41 + y * 11 + 37) % 256) as u8;
      let b = ((x * 7 + y * 23 + 71) % 256) as u8;
      image.put_pixel(x, y, Rgba([r, g, b, 255]));
    }
  }

  image.save(path).expect("save noisy png");
}

fn create_scratched_png(path: &std::path::Path) {
  let width = 120;
  let height = 100;
  let mut image = RgbaImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let base = ((x * 3 + y * 2 + 90) % 40 + 90) as u8;
      image.put_pixel(x, y, Rgba([base, base, base, 255]));
    }
  }

  for y in 0..height {
    image.put_pixel(60, y, Rgba([255, 255, 255, 255]));
  }

  image.save(path).expect("save scratched png");
}

fn create_low_res_edge_chart(path: &std::path::Path) {
  let width = 48;
  let height = 32;
  let mut image = RgbaImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let mut value: u8 = if x < width / 2 { 30 } else { 220 };

      if x % 7 == 0 {
        value = value.saturating_add(18);
      }

      if y % 6 == 0 {
        value = value.saturating_sub(12);
      }

      if (14..=18).contains(&x) && (8..=24).contains(&y) {
        value = 250;
      }

      image.put_pixel(x, y, Rgba([value, value, value, 255]));
    }
  }

  image.save(path).expect("save edge chart png");
}

fn luma(pixel: [u8; 4]) -> f32 {
  (pixel[0] as f32 * 0.299) + (pixel[1] as f32 * 0.587) + (pixel[2] as f32 * 0.114)
}

fn edge_energy(image: &RgbaImage) -> f64 {
  let (width, height) = image.dimensions();
  if width < 3 || height < 3 {
    return 0.0;
  }

  let mut sum = 0.0_f64;
  let mut count = 0_u64;

  for y in 1..(height - 1) {
    for x in 1..(width - 1) {
      let left = luma(image.get_pixel(x - 1, y).0);
      let right = luma(image.get_pixel(x + 1, y).0);
      let up = luma(image.get_pixel(x, y - 1).0);
      let down = luma(image.get_pixel(x, y + 1).0);

      let gx = (right - left).abs();
      let gy = (down - up).abs();
      sum += ((gx + gy) * 0.5) as f64;
      count = count.saturating_add(1);
    }
  }

  if count == 0 {
    return 0.0;
  }

  sum / count as f64
}

fn create_denoise_benchmark_png(path: &std::path::Path) {
  let width = 128;
  let height = 96;
  let mut image = RgbaImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let mut base: i32 = if x < 64 { 96 } else { 176 };

      if x > 80 {
        if x % 8 < 2 {
          base = 228;
        } else if x % 8 > 5 {
          base = 54;
        }
      }

      let noise = ((x * 37 + y * 17 + (x * y) % 31) % 23) as i32 - 11;
      let value = (base + noise).clamp(0, 255) as u8;
      image.put_pixel(x, y, Rgba([value, value, value, 255]));
    }
  }

  image.save(path).expect("save denoise benchmark image");
}

fn region_neighbor_delta(
  image: &RgbaImage,
  x0: u32,
  x1: u32,
  y0: u32,
  y1: u32,
) -> f64 {
  if x1 <= x0 + 1 || y1 <= y0 + 1 {
    return 0.0;
  }

  let mut sum = 0.0_f64;
  let mut count = 0_u64;

  for y in y0..(y1 - 1) {
    for x in x0..(x1 - 1) {
      let center = luma(image.get_pixel(x, y).0);
      let right = luma(image.get_pixel(x + 1, y).0);
      let down = luma(image.get_pixel(x, y + 1).0);

      sum += (((center - right).abs() + (center - down).abs()) * 0.5) as f64;
      count = count.saturating_add(1);
    }
  }

  if count == 0 {
    return 0.0;
  }

  sum / count as f64
}

fn vertical_step_contrast(
  image: &RgbaImage,
  left_start: u32,
  left_end: u32,
  right_start: u32,
  right_end: u32,
  y0: u32,
  y1: u32,
) -> f64 {
  let mut sum = 0.0_f64;
  let mut rows = 0_u64;

  for y in y0..y1 {
    let mut left_acc = 0.0_f32;
    let mut right_acc = 0.0_f32;
    let mut left_count = 0_u32;
    let mut right_count = 0_u32;

    for x in left_start..left_end {
      left_acc += luma(image.get_pixel(x, y).0);
      left_count += 1;
    }
    for x in right_start..right_end {
      right_acc += luma(image.get_pixel(x, y).0);
      right_count += 1;
    }

    if left_count == 0 || right_count == 0 {
      continue;
    }

    let left_mean = left_acc / left_count as f32;
    let right_mean = right_acc / right_count as f32;

    sum += (right_mean - left_mean).abs() as f64;
    rows = rows.saturating_add(1);
  }

  if rows == 0 {
    return 0.0;
  }

  sum / rows as f64
}

#[test]
fn rejects_invalid_repair_mode() {
  let processor = RepairProcessor;
  let validation = processor.validate(&json!({"mode": "magic", "strength": 50}));
  assert!(validation.is_err());
}

#[test]
fn rejects_invalid_upscale_factor() {
  let processor = RepairProcessor;
  let validation = processor.validate(
    &json!({"mode": "upscale", "strength": 60, "upscaleFactor": 1}),
  );
  assert!(validation.is_err());
}

#[test]
fn rejects_invalid_upscale_sharpness() {
  let processor = RepairProcessor;
  let validation = processor.validate(
    &json!({"mode": "upscale", "strength": 60, "upscaleFactor": 2, "upscaleSharpness": 0}),
  );
  assert!(validation.is_err());
}

#[test]
fn repairs_with_denoise_mode() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_noisy_png(&input_path);
  let input_image = image::open(&input_path)
    .expect("open input")
    .to_rgba8();

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path: input_path.clone(),
    output_path: output_path.clone(),
    params: json!({"mode": "denoise", "strength": 70}),
  };

  let result = processor.process(&context).expect("denoise should succeed");
  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.message.contains("去噪"));

  let output = result.output_path.expect("output path should exist");
  assert!(output.exists());

  let output_image = image::open(&output)
    .expect("open output")
    .to_rgba8();

  assert_eq!(input_image.dimensions(), output_image.dimensions());

  let changed_pixels = input_image
    .pixels()
    .zip(output_image.pixels())
    .filter(|(left, right)| left.0 != right.0)
    .count();
  assert!(changed_pixels > 0);
}

#[test]
fn denoise_mode_reduces_noise_without_heavy_blur() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input-denoise-benchmark.png");
  let output_path = temp.path().join("output-denoise-benchmark.png");

  create_denoise_benchmark_png(&input_path);
  let before = image::open(&input_path)
    .expect("open denoise benchmark input")
    .to_rgba8();

  let noise_before = region_neighbor_delta(&before, 8, 56, 8, 88);
  let step_before = vertical_step_contrast(&before, 58, 63, 65, 70, 8, 88);

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path,
    params: json!({"mode": "denoise", "strength": 72}),
  };

  let result = processor.process(&context).expect("denoise benchmark should succeed");
  let after = image::open(result.output_path.as_ref().expect("output path"))
    .expect("open denoised output")
    .to_rgba8();

  let noise_after = region_neighbor_delta(&after, 8, 56, 8, 88);
  let step_after = vertical_step_contrast(&after, 58, 63, 65, 70, 8, 88);

  assert!(noise_after < noise_before * 0.82);
  assert!(step_after > step_before * 0.80);
}

#[test]
fn repairs_scratch_line_in_scratch_mode() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_scratched_png(&input_path);
  let before = image::open(&input_path)
    .expect("open input")
    .to_rgba8();

  let x = 60u32;
  let y = 50u32;
  let before_center = before.get_pixel(x, y).0[0] as i32;
  let before_neighbors_avg =
    (before.get_pixel(x - 1, y).0[0] as i32 + before.get_pixel(x + 1, y).0[0] as i32) / 2;

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path,
    params: json!({"mode": "scratch", "strength": 85}),
  };

  let result = processor.process(&context).expect("scratch repair should succeed");
  assert_eq!(result.status, ProcessStatus::Success);

  let output = result.output_path.expect("output path should exist");
  let after = image::open(&output)
    .expect("open output")
    .to_rgba8();

  let after_center = after.get_pixel(x, y).0[0] as i32;

  let before_diff = (before_center - before_neighbors_avg).abs();
  let after_diff = (after_center - before_neighbors_avg).abs();
  assert!(after_diff < before_diff);
}

#[test]
fn auto_mode_runs_successfully() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_scratched_png(&input_path);

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path,
    params: json!({"mode": "auto", "strength": 60}),
  };

  let result = processor.process(&context).expect("auto mode should succeed");

  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.output_path.expect("output path should exist").exists());
}

#[test]
fn upscale_mode_increases_resolution() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input.png");
  let output_path = temp.path().join("output.png");

  create_noisy_png(&input_path);

  let input = image::open(&input_path).expect("open input");

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path,
    params: json!({"mode": "upscale", "strength": 72, "upscaleFactor": 3, "upscaleSharpness": 70}),
  };

  let result = processor.process(&context).expect("upscale mode should succeed");

  assert_eq!(result.status, ProcessStatus::Success);
  assert!(result.message.contains("低分辨率增强"));

  let output_path = result.output_path.expect("output path should exist");
  let output = image::open(&output_path).expect("open output");

  assert_eq!(output.width(), input.width() * 3);
  assert_eq!(output.height(), input.height() * 3);
}

#[test]
fn upscale_mode_improves_edge_clarity_over_plain_resize() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input-edge.png");
  let output_path = temp.path().join("output-edge.png");

  create_low_res_edge_chart(&input_path);

  let input = image::open(&input_path)
    .expect("open input edge chart")
    .to_rgba8();

  let baseline = image::imageops::resize(
    &input,
    input.width() * 3,
    input.height() * 3,
    FilterType::Lanczos3,
  );
  let baseline_energy = edge_energy(&baseline);

  let processor = RepairProcessor;
  let context = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path,
    params: json!({"mode": "upscale", "strength": 80, "upscaleFactor": 3, "upscaleSharpness": 82}),
  };

  let result = processor.process(&context).expect("upscale should succeed");
  let output = image::open(result.output_path.as_ref().expect("output path"))
    .expect("open repaired output")
    .to_rgba8();

  let enhanced_energy = edge_energy(&output);
  assert!(enhanced_energy > baseline_energy * 1.02);
}

#[test]
fn upscale_sharpness_parameter_changes_detail_strength() {
  let temp = tempdir().expect("create temp dir");
  let input_path = temp.path().join("input-sharpness.png");
  let output_low = temp.path().join("output-low.png");
  let output_high = temp.path().join("output-high.png");

  create_low_res_edge_chart(&input_path);

  let processor = RepairProcessor;

  let low_ctx = ProcessContext {
    processor_id: "repair".to_string(),
    input_path: input_path.clone(),
    output_path: output_low,
    params: json!({"mode": "upscale", "strength": 72, "upscaleFactor": 3, "upscaleSharpness": 25}),
  };
  let low_result = processor.process(&low_ctx).expect("low sharpness upscale should succeed");
  let low_img = image::open(low_result.output_path.as_ref().expect("low output path"))
    .expect("open low sharpness image")
    .to_rgba8();

  let high_ctx = ProcessContext {
    processor_id: "repair".to_string(),
    input_path,
    output_path: output_high,
    params: json!({"mode": "upscale", "strength": 72, "upscaleFactor": 3, "upscaleSharpness": 92}),
  };
  let high_result = processor.process(&high_ctx).expect("high sharpness upscale should succeed");
  let high_img = image::open(high_result.output_path.as_ref().expect("high output path"))
    .expect("open high sharpness image")
    .to_rgba8();

  let low_energy = edge_energy(&low_img);
  let high_energy = edge_energy(&high_img);

  assert!(high_energy > low_energy * 1.03);
}
