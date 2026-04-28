use crate::core::processor::{ProcessContext, ProcessError, ProcessResult, Processor, ProcessorDescriptor};
use serde_json::Value;

#[derive(Default)]
pub struct RenameProcessor;

impl Processor for RenameProcessor {
  fn descriptor(&self) -> ProcessorDescriptor {
    ProcessorDescriptor {
      id: "rename".to_string(),
      display_name: "批量重命名".to_string(),
      enabled: true,
      notes: "仅修改输出文件名，不改变图片内容。".to_string(),
    }
  }

  fn validate(&self, _params: &Value) -> Result<(), ProcessError> {
    Ok(())
  }

  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError> {
    if context.input_path == context.output_path {
      return Ok(ProcessResult::skipped(
        "输出路径与输入路径相同，已跳过。",
      ));
    }

    if let Some(parent) = context.output_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(&context.input_path, &context.output_path)?;
    Ok(ProcessResult::success(
      "重命名完成。",
      context.output_path.clone(),
    ))
  }
}
