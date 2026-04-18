use crate::core::processor::{
  ProcessContext,
  ProcessError,
  ProcessResult,
  Processor,
  ProcessorDescriptor,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepairParams {
  mode: String,
  strength: u8,
}

#[derive(Default)]
pub struct RepairProcessor;

impl Processor for RepairProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "repair".to_string(),
      display_name: "图像修复".to_string(),
      enabled: false,
      notes: "扩展预留：后续将支持去噪、划痕修复与内容补全。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = serde_json::from_value::<RepairParams>(params.clone())?;
    let mode = parsed.mode.to_lowercase();

    if !["auto", "denoise", "scratch"].contains(&mode.as_str()) {
      return Err(ProcessError::Validation(
        "mode 不受支持，请使用 auto/denoise/scratch。".to_string(),
      ));
    }

    if !(1..=100).contains(&parsed.strength) {
      return Err(ProcessError::Validation(
        "strength 必须在 1 到 100 之间。".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, _context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    Err(ProcessError::Unsupported(
      "图像修复算法尚未实现。".to_string(),
    ))
  }
}
