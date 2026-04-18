use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessContext {
  pub processor_id: String,
  pub input_path: PathBuf,
  pub output_path: PathBuf,
  #[serde(default)]
  pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
  pub width: u32,
  pub height: u32,
  pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
  Success,
  Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResult {
  pub status: ProcessStatus,
  pub message: String,
  pub output_path: Option<PathBuf>,
  pub input_metadata: Option<ImageMetadata>,
  pub output_metadata: Option<ImageMetadata>,
}

impl ProcessResult {
  pub fn success(message: impl Into<String>, output_path: PathBuf) -> Self {
    Self {
      status: ProcessStatus::Success,
      message: message.into(),
      output_path: Some(output_path),
      input_metadata: None,
      output_metadata: None,
    }
  }

  pub fn skipped(message: impl Into<String>) -> Self {
    Self {
      status: ProcessStatus::Skipped,
      message: message.into(),
      output_path: None,
      input_metadata: None,
      output_metadata: None,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorDescriptor {
  pub id: String,
  pub display_name: String,
  pub enabled: bool,
  pub notes: String,
}

pub trait Processor: Send + Sync {
  fn descriptor(&self) -> ProcessorDescriptor;
  fn validate(&self, params: &Value) -> Result<(), ProcessError>;
  fn process(&self, context: &ProcessContext) -> Result<ProcessResult, ProcessError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
  #[error("参数校验失败: {0}")]
  Validation(String),
  #[error("功能暂未实现: {0}")]
  Unsupported(String),
  #[error("文件系统异常: {0}")]
  Io(#[from] std::io::Error),
  #[error("图像处理异常: {0}")]
  Image(#[from] image::ImageError),
  #[error("参数解析异常: {0}")]
  Json(#[from] serde_json::Error),
  #[error("内部异常: {0}")]
  Internal(String),
}

impl ProcessError {
  pub fn user_message(&self) -> String {
    match self {
      Self::Validation(msg)
      | Self::Unsupported(msg)
      | Self::Internal(msg) => msg.clone(),
      Self::Io(err) => format!("文件读写失败: {err}"),
      Self::Image(err) => format!("图片解析失败: {err}"),
      Self::Json(err) => format!("参数格式错误: {err}"),
    }
  }
}
