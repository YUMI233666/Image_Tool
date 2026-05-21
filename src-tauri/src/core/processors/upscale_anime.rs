use crate::core::processor::{ProcessContext, ProcessError, ProcessResult, Processor, ProcessorDescriptor};
use crate::core::upscale_runner::run_upscale_sidecar;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleAnimeParams {
  #[serde(default = "default_scale")]
  scale: u8,
  #[serde(default = "default_denoise")]
  denoise_level: u8,
}

fn default_scale() -> u8 {
  2
}

fn default_denoise() -> u8 {
  3
}

#[derive(Default)]
pub struct UpscaleAnimeProcessor;

impl UpscaleAnimeProcessor {
  fn parse_params(&self, params: &Value) -> Result<UpscaleAnimeParams, ProcessError> {
    if params.is_null() {
      return Ok(UpscaleAnimeParams {
        scale: default_scale(),
        denoise_level: default_denoise(),
      });
    }

    Ok(serde_json::from_value::<UpscaleAnimeParams>(params.clone())?)
  }

  fn validate_params(&self, params: &UpscaleAnimeParams) -> Result<(), ProcessError> {
    if params.scale != 2 && params.scale != 4 {
      return Err(ProcessError::Validation(
        "倍率仅支持 2x 或 4x。".to_string(),
      ));
    }

    if !(1..=3).contains(&params.denoise_level) {
      return Err(ProcessError::Validation(
        "降噪等级仅支持 1-3。".to_string(),
      ));
    }

    Ok(())
  }
}

impl Processor for UpscaleAnimeProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "upscale-anime".to_string(),
      display_name: "二次元超分".to_string(),
      enabled: true,
      notes: "使用 realcugan-ncnn-vulkan 进行二次元风格超分。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = self.parse_params(params)?;
    self.validate_params(&parsed)
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    let parsed = self.parse_params(&context.params)?;
    self.validate_params(&parsed)?;

    let output_path = run_upscale_sidecar(
      &context.input_path,
      &context.output_path,
      parsed.scale,
      parsed.denoise_level,
    )?;

    Ok(ProcessResult::success("超分完成。", output_path))
  }
}
