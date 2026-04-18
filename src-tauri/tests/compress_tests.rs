use art_tool_lib::core::processor::{ProcessContext, ProcessStatus, Processor};
use art_tool_lib::core::processors::compress::CompressProcessor;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use image::codecs::jpeg::JpegEncoder;
use serde_json::json;
use std::fs::File;
use std::io::BufWriter;
use tempfile::tempdir;

fn create_high_quality_jpg(path: &std::path::Path) {
  let width = 260;
  let height = 180;
  let mut image = RgbImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let r = ((x * 37 + y * 17) % 256) as u8;
      let g = ((x * 13 + y * 29) % 256) as u8;
      let b = ((x * 53 + y * 7) % 256) as u8;
      image.put_pixel(x, y, Rgb([r, g, b]));
    }
  }

  let writer = BufWriter::new(File::create(path).expect("create source jpg"));
  let mut encoder = JpegEncoder::new_with_quality(writer, 96);
  encoder
    .encode(&image, width, height, image::ColorType::Rgb8.into())
    .expect("encode source jpg");
}

fn create_sample_bmp(path: &std::path::Path) {
  let mut image = RgbImage::new(64, 64);
  for y in 0..64 {
    for x in 0..64 {
      image.put_pixel(x, y, Rgb([(x * 3) as u8, (y * 3) as u8, (x + y) as u8]));
    }
  }
  image.save(path).expect("save bmp");
}

fn create_noisy_png(path: &std::path::Path) {
  let width = 300;
  let height = 220;
  let mut image = RgbaImage::new(width, height);

  for y in 0..height {
    for x in 0..width {
      let r = ((x * 29 + y * 13 + 17) % 256) as u8;
      let g = ((x * 7 + y * 31 + 73) % 256) as u8;
      let b = ((x * 19 + y * 11 + 191) % 256) as u8;
      let a = if (x + y) % 9 == 0 { 220 } else { 255 };
      image.put_pixel(x, y, Rgba([r, g, b, a]));
    }
  }

  image.save(path).expect("save noisy png");
}

#[test]
fn compresses_jpeg_with_smaller_size() {
  let tmp = tempdir().expect("create temp dir");
  let input_path = tmp.path().join("input.jpg");
  let output_path = tmp.path().join("output.jpg");

  create_high_quality_jpg(&input_path);
  let input_size = std::fs::metadata(&input_path)
    .expect("read input metadata")
    .len();

  let processor = CompressProcessor;
  let context = ProcessContext {
    processor_id: "compress".to_string(),
    input_path: input_path.clone(),
    output_path: output_path.clone(),
    params: json!({"quality": 28, "mode": "lossy"}),
  };

  let result = processor.process(&context).expect("compress should succeed");

  assert_eq!(result.status, ProcessStatus::Success);
  let final_path = result.output_path.expect("output path should exist");
  assert!(final_path.exists());

  let output_size = std::fs::metadata(&final_path)
    .expect("read output metadata")
    .len();
  assert!(output_size < input_size);
}

#[test]
fn rejects_invalid_compress_mode() {
  let processor = CompressProcessor;
  let validation = processor.validate(&json!({"quality": 70, "mode": "super"}));

  assert!(validation.is_err());
}

#[test]
fn skips_unsupported_bmp_compression() {
  let tmp = tempdir().expect("create temp dir");
  let input_path = tmp.path().join("input.bmp");
  let output_path = tmp.path().join("output.bmp");

  create_sample_bmp(&input_path);

  let processor = CompressProcessor;
  let context = ProcessContext {
    processor_id: "compress".to_string(),
    input_path,
    output_path,
    params: json!({"quality": 80, "mode": "balanced"}),
  };

  let result = processor.process(&context).expect("compress should return result");

  assert_eq!(result.status, ProcessStatus::Skipped);
  assert!(result.message.contains("暂不支持 BMP/TIFF"));
}

#[test]
fn compresses_png_in_lossy_mode() {
  let tmp = tempdir().expect("create temp dir");
  let input_path = tmp.path().join("input.png");
  let output_path = tmp.path().join("output.png");

  create_noisy_png(&input_path);

  let input_size = std::fs::metadata(&input_path)
    .expect("read input metadata")
    .len();

  let processor = CompressProcessor;
  let context = ProcessContext {
    processor_id: "compress".to_string(),
    input_path: input_path.clone(),
    output_path: output_path.clone(),
    params: json!({"quality": 40, "mode": "lossy"}),
  };

  let result = processor.process(&context).expect("compress should return");

  assert_eq!(result.status, ProcessStatus::Success);
  let final_path = result.output_path.expect("output path should exist");
  assert!(final_path.exists());

  let output_size = std::fs::metadata(&final_path)
    .expect("read output metadata")
    .len();
  assert!(output_size < input_size);
}
