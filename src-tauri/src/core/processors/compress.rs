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
struct CompressParams {
  quality: u8,
}

#[derive(Default)]
pub struct CompressProcessor;

impl Processor for CompressProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "compress".to_string(),
      display_name: "图像压缩".to_string(),
      enabled: false,
      notes: "扩展预留：后续将支持有损/无损压缩策略。".to_string(),
    }
  }

  fn validate(&self, params: &Value) -> Result<(), ProcessError> {
    let parsed = serde_json::from_value::<CompressParams>(params.clone())?;

    if !(1..=100).contains(&parsed.quality) {
      return Err(ProcessError::Validation(
        "quality 必须在 1 到 100 之间。".to_string(),
      ));
    }

    Ok(())
  }

  fn process(&self, _context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    Err(ProcessError::Unsupported(
      "图像压缩算法尚未实现。".to_string(),
    ))
  }
}
