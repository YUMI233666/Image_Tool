use crate::core::processor::{
  ImageMetadata,
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use image::{
  imageops,
  imageops::FilterType,
  DynamicImage,
  GenericImageView,
  ImageFormat,
  ImageReader,
  Rgba,
  RgbaImage,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

const MAX_UPSCALE_EDGE: u32 = 8192;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepairParams {
  #[serde(default = "default_mode")]
  mode: String,
  #[serde(default = "default_strength")]
  strength: u8,
  #[serde(default = "default_upscale_factor")]
  upscale_factor: u8,
  #[serde(default = "default_upscale_sharpness")]
  upscale_sharpness: u8,
}

fn default_mode() -> String {
  "auto".to_string()
}

fn default_strength() -> u8 {
  50
}

fn default_upscale_factor() -> u8 {
  2
}

fn default_upscale_sharpness() -> u8 {
  70
}

impl Default for RepairParams {
  fn default() -> Self {
    Self {
      mode: default_mode(),
      strength: default_strength(),
      upscale_factor: default_upscale_factor(),
      upscale_sharpness: default_upscale_sharpness(),
    }
  }
}

fn normalize_mode(raw: &str) -> Option<&'static str> {
  match raw.trim().to_ascii_lowercase().as_str() {
    "auto" => Some("auto"),
    "denoise" => Some("denoise"),
    "scratch" => Some("scratch"),
    "upscale" => Some("upscale"),
    _ => None,
  }
}

fn extension_of(path: &Path) -> Option<String> {
  path
    .extension()
    .and_then(|ext| ext.to_str())
    .map(|ext| ext.to_ascii_lowercase())
}

fn to_luma(pixel: &Rgba<u8>) -> i32 {
  let [r, g, b, _] = pixel.0;
  ((r as i32 * 299) + (g as i32 * 587) + (b as i32 * 114)) / 1000
}

fn blend_channel(base: u8, overlay: u8, weight: f32) -> u8 {
  let w = weight.clamp(0.0, 1.0);
  let value = (base as f32 * (1.0 - w)) + (overlay as f32 * w);
  value.round().clamp(0.0, 255.0) as u8
}

fn blend_images(base: &RgbaImage, overlay: &RgbaImage, weight: f32) -> RgbaImage {
  let (width, height) = base.dimensions();
  let mut output = base.clone();

  for y in 0..height {
    for x in 0..width {
      let b = base.get_pixel(x, y);
      let o = overlay.get_pixel(x, y);
      output.put_pixel(
        x,
        y,
        Rgba([
          blend_channel(b.0[0], o.0[0], weight),
          blend_channel(b.0[1], o.0[1], weight),
          blend_channel(b.0[2], o.0[2], weight),
          b.0[3],
        ]),
      );
    }
  }

  output
}

fn count_changed_pixels(before: &RgbaImage, after: &RgbaImage) -> u64 {
  before
    .pixels()
    .zip(after.pixels())
    .filter(|(left, right)| left.0 != right.0)
    .count() as u64
}

fn median_of_9(mut values: [u8; 9]) -> u8 {
  values.sort_unstable();
  values[4]
}

fn denoise_channel(
  center: u8,
  median: u8,
  blur: u8,
  strength_norm: f32,
  edge_weight: f32,
  noise_gate: f32,
  texture_span: f32,
) -> u8 {
  let mix_to_blur = 0.26 + 0.26 * strength_norm;
  let target = blend_channel(median, blur, mix_to_blur);

  let texture_guard = 1.0 - ((texture_span - 30.0) / 120.0).clamp(0.0, 1.0) * 0.50;
  let edge_guard = 1.0 - edge_weight * 0.78;

  let base_weight = 0.06 + 0.19 * strength_norm;
  let adaptive_weight = (1.00 * strength_norm * noise_gate * texture_guard * edge_guard)
    .clamp(0.0, 0.86);

  let mut final_weight = (base_weight + adaptive_weight).clamp(0.0, 0.90);
  if edge_weight > 0.55 {
    final_weight *= 0.58;
  }
  if texture_span > 90.0 {
    final_weight *= 0.72;
  }
  if noise_gate < 0.08 && edge_weight < 0.20 {
    final_weight *= 0.60;
  }

  blend_channel(center, target, final_weight)
}

fn apply_denoise(source: &RgbaImage, strength: u8) -> RgbaImage {
  if strength <= 3 {
    return source.clone();
  }

  let (width, height) = source.dimensions();
  let strength_norm = strength as f32 / 100.0;
  let blurred = imageops::blur(source, 0.50 + 1.55 * strength_norm);
  let edge_weights = build_edge_weight_map(source);

  let mut output = source.clone();
  let index = |x: u32, y: u32| -> usize { (y as usize) * width as usize + x as usize };

  for y in 0..height {
    for x in 0..width {
      let mut rv = [0u8; 9];
      let mut gv = [0u8; 9];
      let mut bv = [0u8; 9];
      let mut lv = [0u8; 9];

      let mut k = 0usize;
      let mut luma_min = 255u8;
      let mut luma_max = 0u8;

      for oy in 0..3 {
        for ox in 0..3 {
          let sx = (x + ox).saturating_sub(1).min(width.saturating_sub(1));
          let sy = (y + oy).saturating_sub(1).min(height.saturating_sub(1));
          let p = source.get_pixel(sx, sy).0;

          rv[k] = p[0];
          gv[k] = p[1];
          bv[k] = p[2];

          let local_luma = luma_from_rgb_u8(p[0], p[1], p[2]).round().clamp(0.0, 255.0) as u8;
          lv[k] = local_luma;
          luma_min = luma_min.min(local_luma);
          luma_max = luma_max.max(local_luma);
          k += 1;
        }
      }

      let median_r = median_of_9(rv);
      let median_g = median_of_9(gv);
      let median_b = median_of_9(bv);
      let median_luma = median_of_9(lv) as f32;

      let center = source.get_pixel(x, y).0;
      let blur = blurred.get_pixel(x, y).0;
      let edge_weight = edge_weights[index(x, y)];

      let center_luma = luma_from_rgb_u8(center[0], center[1], center[2]);
      let noise_delta = (center_luma - median_luma).abs();
      let chroma_noise = (
        (center[0] as i32 - median_r as i32).abs()
          .max((center[1] as i32 - median_g as i32).abs())
          .max((center[2] as i32 - median_b as i32).abs())
      ) as f32;
      let luma_gate = ((noise_delta - 0.6) / 10.0).clamp(0.0, 1.0);
      let chroma_gate = ((chroma_noise - 0.8) / 14.0).clamp(0.0, 1.0);
      let noise_gate = luma_gate.max(chroma_gate);
      let texture_span = (luma_max as f32 - luma_min as f32).abs();

      output.put_pixel(
        x,
        y,
        Rgba([
          denoise_channel(
            center[0],
            median_r,
            blur[0],
            strength_norm,
            edge_weight,
            noise_gate,
            texture_span,
          ),
          denoise_channel(
            center[1],
            median_g,
            blur[1],
            strength_norm,
            edge_weight,
            noise_gate,
            texture_span,
          ),
          denoise_channel(
            center[2],
            median_b,
            blur[2],
            strength_norm,
            edge_weight,
            noise_gate,
            texture_span,
          ),
          center[3],
        ]),
      );
    }
  }

  output
}

fn average_two_pixels(left: Rgba<u8>, right: Rgba<u8>) -> Rgba<u8> {
  let avg = |a: u8, b: u8| ((a as u16 + b as u16) / 2) as u8;
  Rgba([
    avg(left.0[0], right.0[0]),
    avg(left.0[1], right.0[1]),
    avg(left.0[2], right.0[2]),
    avg(left.0[3], right.0[3]),
  ])
}

fn scratch_pass(
  source: &RgbaImage,
  brightness_threshold: i32,
  continuity_tolerance: i32,
) -> (RgbaImage, u64) {
  let (width, height) = source.dimensions();
  if width < 3 || height < 3 {
    return (source.clone(), 0);
  }

  let mut output = source.clone();
  let mut changed = 0u64;

  for y in 1..(height - 1) {
    for x in 1..(width - 1) {
      let current = *source.get_pixel(x, y);
      let left = *source.get_pixel(x - 1, y);
      let right = *source.get_pixel(x + 1, y);
      let up = *source.get_pixel(x, y - 1);
      let down = *source.get_pixel(x, y + 1);

      let current_luma = to_luma(&current);
      let horizontal_target = (to_luma(&left) + to_luma(&right)) / 2;
      let vertical_target = (to_luma(&up) + to_luma(&down)) / 2;

      let horizontal_continuity = (to_luma(&left) - to_luma(&right)).abs();
      let vertical_continuity = (to_luma(&up) - to_luma(&down)).abs();

      let horizontal_anomaly =
        (current_luma - horizontal_target).abs() >= brightness_threshold
          && horizontal_continuity <= continuity_tolerance;

      let vertical_anomaly =
        (current_luma - vertical_target).abs() >= brightness_threshold
          && vertical_continuity <= continuity_tolerance;

      if !horizontal_anomaly && !vertical_anomaly {
        continue;
      }

      let replacement = if horizontal_anomaly && vertical_anomaly {
        if horizontal_continuity <= vertical_continuity {
          average_two_pixels(left, right)
        } else {
          average_two_pixels(up, down)
        }
      } else if horizontal_anomaly {
        average_two_pixels(left, right)
      } else {
        average_two_pixels(up, down)
      };

      output.put_pixel(x, y, replacement);
      changed = changed.saturating_add(1);
    }
  }

  (output, changed)
}

fn apply_scratch_repair(source: &RgbaImage, strength: u8) -> (RgbaImage, u64) {
  let mut working = source.clone();
  let mut total_changed = 0u64;

  let passes = if strength >= 80 {
    3
  } else if strength >= 45 {
    2
  } else {
    1
  };

  let base_threshold = (62 - (strength as i32 / 2)).clamp(14, 60);
  let continuity_tolerance = (base_threshold / 2 + 8).clamp(10, 36);

  for pass_index in 0..passes {
    let current_threshold = (base_threshold - pass_index * 4).clamp(10, 60);
    let (next, changed) = scratch_pass(&working, current_threshold, continuity_tolerance);
    working = next;
    total_changed = total_changed.saturating_add(changed);
  }

  let blend_weight = 0.35 + 0.55 * (strength as f32 / 100.0);
  let blended = blend_images(source, &working, blend_weight);
  (blended, total_changed)
}

fn upscale_dimensions(width: u32, height: u32, factor: u8) -> (u32, u32) {
  let f = factor.clamp(2, 4) as u32;

  let target_width = width.saturating_mul(f).clamp(1, MAX_UPSCALE_EDGE);
  let target_height = height.saturating_mul(f).clamp(1, MAX_UPSCALE_EDGE);

  (target_width, target_height)
}

fn luma_from_rgb_u8(r: u8, g: u8, b: u8) -> f32 {
  (r as f32 * 0.299) + (g as f32 * 0.587) + (b as f32 * 0.114)
}

fn build_edge_weight_map(image: &RgbaImage) -> Vec<f32> {
  let (width, height) = image.dimensions();
  let total = (width as usize).saturating_mul(height as usize);

  if total == 0 {
    return Vec::new();
  }

  let index = |x: u32, y: u32| -> usize { (y as usize) * width as usize + x as usize };

  let mut luma = vec![0.0_f32; total];
  for y in 0..height {
    for x in 0..width {
      let pixel = image.get_pixel(x, y).0;
      luma[index(x, y)] = luma_from_rgb_u8(pixel[0], pixel[1], pixel[2]);
    }
  }

  let mut edges = vec![0.0_f32; total];
  for y in 0..height {
    for x in 0..width {
      let xl = x.saturating_sub(1);
      let xr = (x + 1).min(width.saturating_sub(1));
      let yu = y.saturating_sub(1);
      let yd = (y + 1).min(height.saturating_sub(1));

      let gx = (luma[index(xr, y)] - luma[index(xl, y)]).abs();
      let gy = (luma[index(x, yd)] - luma[index(x, yu)]).abs();

      let gradient = (gx + gy) / 255.0;
      // Compress to [0,1] and bias toward preserving strong edges.
      let edge_weight = (gradient / 1.65).clamp(0.0, 1.0).powf(0.72);
      edges[index(x, y)] = edge_weight;
    }
  }

  edges
}

fn enhance_upscale_channel(
  base: u8,
  micro_blur: u8,
  local_blur: u8,
  strength_norm: f32,
  sharpness_norm: f32,
  edge_weight: f32,
) -> u8 {
  let base_f = base as f32;
  let micro_detail = (base_f - micro_blur as f32).clamp(-56.0, 56.0);
  let local_detail = (base_f - local_blur as f32).clamp(-34.0, 34.0);

  let detail_gate = ((micro_detail.abs() - 1.0) / 10.0).clamp(0.0, 1.0);
  let edge_gain = 0.95 + edge_weight * 1.35;
  let detail_gain = (0.30 + 1.50 * sharpness_norm) * edge_gain;
  let contrast_gain =
    (0.06 + 0.18 * strength_norm + 0.14 * sharpness_norm) * (0.80 + edge_weight * 0.70);

  let mut value =
    base_f + micro_detail * detail_gain * detail_gate + local_detail * contrast_gain;

  if edge_weight < 0.2 {
    value = base_f + (value - base_f) * (0.86 + 0.22 * sharpness_norm);
  }

  value.round().clamp(0.0, 255.0) as u8
}

fn boost_fine_detail_channel(base: u8, blurred: u8, gain: f32) -> u8 {
  let base_f = base as f32;
  let fine = (base_f - blurred as f32).clamp(-24.0, 24.0);
  let value = base_f + fine * gain;
  value.round().clamp(0.0, 255.0) as u8
}

fn apply_upscale_enhance(
  source: &RgbaImage,
  strength: u8,
  upscale_factor: u8,
  upscale_sharpness: u8,
) -> (RgbaImage, u64) {
  let (target_width, target_height) =
    upscale_dimensions(source.width(), source.height(), upscale_factor);
  let strength_norm = strength as f32 / 100.0;
  let sharpness_norm = upscale_sharpness as f32 / 100.0;

  let prefiltered = if strength >= 92 {
    apply_denoise(source, ((strength - 70) / 2).max(8))
  } else {
    source.clone()
  };

  let upscaled_lanczos = imageops::resize(
    &prefiltered,
    target_width,
    target_height,
    FilterType::Lanczos3,
  );
  let upscaled_catmull = imageops::resize(
    &prefiltered,
    target_width,
    target_height,
    FilterType::CatmullRom,
  );

  let interpolation_blend = (0.18 + 0.22 * strength_norm + 0.28 * sharpness_norm).clamp(0.0, 0.82);
  let upscaled = blend_images(&upscaled_lanczos, &upscaled_catmull, interpolation_blend);

  let edge_weights = build_edge_weight_map(&upscaled);

  let micro_blurred = imageops::blur(&upscaled, 0.42 + 0.85 * strength_norm);
  let local_blurred = imageops::blur(&upscaled, 1.20 + 1.65 * strength_norm);

  let mut enhanced = upscaled.clone();
  let index = |x: u32, y: u32| -> usize { (y as usize) * target_width as usize + x as usize };

  for y in 0..target_height {
    for x in 0..target_width {
      let base = upscaled.get_pixel(x, y);
      let micro = micro_blurred.get_pixel(x, y);
      let local = local_blurred.get_pixel(x, y);
      let edge_weight = edge_weights[index(x, y)];

      enhanced.put_pixel(
        x,
        y,
        Rgba([
          enhance_upscale_channel(
            base.0[0],
            micro.0[0],
            local.0[0],
            strength_norm,
            sharpness_norm,
            edge_weight,
          ),
          enhance_upscale_channel(
            base.0[1],
            micro.0[1],
            local.0[1],
            strength_norm,
            sharpness_norm,
            edge_weight,
          ),
          enhance_upscale_channel(
            base.0[2],
            micro.0[2],
            local.0[2],
            strength_norm,
            sharpness_norm,
            edge_weight,
          ),
          base.0[3],
        ]),
      );
    }
  }

  let fine_blurred = imageops::blur(&enhanced, 0.55 + 0.55 * strength_norm);
  let mut refined = enhanced.clone();

  for y in 0..target_height {
    for x in 0..target_width {
      let base = enhanced.get_pixel(x, y);
      let soft = fine_blurred.get_pixel(x, y);
      let edge_weight = edge_weights[index(x, y)];
      let refine_gain =
        (0.06 + 0.18 * strength_norm + 0.34 * sharpness_norm) * (0.65 + edge_weight * 0.85);

      refined.put_pixel(
        x,
        y,
        Rgba([
          boost_fine_detail_channel(base.0[0], soft.0[0], refine_gain),
          boost_fine_detail_channel(base.0[1], soft.0[1], refine_gain),
          boost_fine_detail_channel(base.0[2], soft.0[2], refine_gain),
          base.0[3],
        ]),
      );
    }
  }

  let changed = count_changed_pixels(&upscaled, &refined);
  (refined, changed)
}

fn save_repaired_image(path: &Path, image: &RgbaImage) -> Result<(), ProcessError> {
  let output = match extension_of(path).as_deref() {
    Some("jpg") | Some("jpeg") => DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(image.clone()).to_rgb8()),
    _ => DynamicImage::ImageRgba8(image.clone()),
  };

  let format = match extension_of(path).as_deref() {
    Some("jpg") | Some("jpeg") => Some(ImageFormat::Jpeg),
    Some("png") => Some(ImageFormat::Png),
    Some("webp") => Some(ImageFormat::WebP),
    Some("bmp") => Some(ImageFormat::Bmp),
    Some("tiff") => Some(ImageFormat::Tiff),
    _ => None,
  };

  if let Some(target_format) = format {
    output.save_with_format(path, target_format)?;
  } else {
    output.save(path)?;
  }

  Ok(())
}

#[derive(Default)]
pub struct RepairProcessor;

impl Processor for RepairProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "repair".to_string(),
      display_name: "图像修复".to_string(),
      enabled: true,
      notes: "支持自动修复、去噪、轻度划痕修复与低分辨率增强（独立锐化强度）。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = if params.is_null() {
      RepairParams::default()
    } else {
      serde_json::from_value::<RepairParams>(params.clone())?
    };
    let mode = parsed.mode.to_lowercase();

    if normalize_mode(&mode).is_none() {
      return Err(ProcessError::Validation(
        "mode 不受支持，请使用 auto/denoise/scratch/upscale。".to_string(),
      ));
    }

    if !(1..=100).contains(&parsed.strength) {
      return Err(ProcessError::Validation(
        "strength 必须在 1 到 100 之间。".to_string(),
      ));
    }

    if !(2..=4).contains(&parsed.upscale_factor) {
      return Err(ProcessError::Validation(
        "upscaleFactor 必须在 2 到 4 之间。".to_string(),
      ));
    }

    if !(1..=100).contains(&parsed.upscale_sharpness) {
      return Err(ProcessError::Validation(
        "upscaleSharpness 必须在 1 到 100 之间。".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let params = if context.params.is_null() {
      RepairParams::default()
    } else {
      serde_json::from_value::<RepairParams>(context.params.clone())?
    };

    let mode = normalize_mode(&params.mode).ok_or_else(|| {
      ProcessError::Validation(
        "mode 不受支持，请使用 auto/denoise/scratch/upscale。".to_string(),
      )
    })?;

    let source_image = ImageReader::open(&context.input_path)?
      .with_guessed_format()?
      .decode()?;
    let source_rgba = source_image.to_rgba8();
    let (width, height) = source_image.dimensions();

    let (repaired, changed_pixels, mode_label) = match mode {
      "denoise" => {
        let first_pass = apply_denoise(&source_rgba, params.strength);
        let denoised = if params.strength >= 55 {
          apply_denoise(&first_pass, (params.strength / 2).max(28))
        } else {
          first_pass
        };
        let changed = count_changed_pixels(&source_rgba, &denoised);
        (denoised, changed, "去噪")
      }
      "scratch" => {
        let (fixed, changed) = apply_scratch_repair(&source_rgba, params.strength);
        (fixed, changed, "划痕修复")
      }
      "upscale" => {
        let (enhanced, changed) =
          apply_upscale_enhance(
            &source_rgba,
            params.strength,
            params.upscale_factor,
            params.upscale_sharpness,
          );
        (enhanced, changed, "低分辨率增强")
      }
      _ => {
        let denoise_strength = params.strength.min(75);
        let denoised = apply_denoise(&source_rgba, denoise_strength);
        let scratch_strength = ((params.strength as u16 * 3) / 4 + 20).min(100) as u8;
        let (fixed, scratch_changed) = apply_scratch_repair(&denoised, scratch_strength);
        let total_changed = count_changed_pixels(&source_rgba, &fixed).max(scratch_changed);
        (fixed, total_changed, "自动")
      }
    };

    if let Some(parent) = context.output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    save_repaired_image(&context.output_path, &repaired)?;

    let (output_width, output_height) = repaired.dimensions();

    let message = if mode == "upscale" {
      format!(
        "图像修复完成（模式: {mode_label}，修复强度: {}，锐化强度: {}，放大: {}x，分辨率: {}x{} -> {}x{}，锐化像素: {}）。",
        params.strength,
        params.upscale_sharpness,
        params.upscale_factor.clamp(2, 4),
        width,
        height,
        output_width,
        output_height,
        changed_pixels
      )
    } else {
      format!(
        "图像修复完成（模式: {mode_label}，强度: {}，调整像素: {}）。",
        params.strength,
        changed_pixels
      )
    };

    let mut result = ProcessResult::success(message, context.output_path.clone());

    result.input_metadata = Some(ImageMetadata {
      width,
      height,
      format: extension_of(&context.input_path),
    });
    result.output_metadata = Some(ImageMetadata {
      width: output_width,
      height: output_height,
      format: extension_of(&context.output_path),
    });

    Ok(result)
  }
}
