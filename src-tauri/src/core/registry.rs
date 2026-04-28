use crate::core::processor::{Processor, ProcessorDescriptor};
use crate::core::processors::{
  compress::CompressProcessor,
  format_convert::FormatConvertProcessor,
  rename::RenameProcessor,
  repair::RepairProcessor,
  resolution_transform::ResolutionTransformProcessor,
  trim_transparent::TrimTransparentProcessor,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ProcessorRegistry {
  processors: HashMap<String, Arc<dyn Processor>>,
}

impl ProcessorRegistry {
  pub fn new() -> Self {
    Self {
      processors: HashMap::new(),
    }
  }

  pub fn register<P>(&mut self, processor: P)
  where
    P: Processor + 'static,
  {
    let descriptor = processor.descriptor();
    self
      .processors
      .insert(descriptor.id, Arc::new(processor) as Arc<dyn Processor>);
  }

  pub fn get(&self, processor_id: &str) -> Option<Arc<dyn Processor>> {
    self.processors.get(processor_id).cloned()
  }

  pub fn descriptors(&self) -> Vec<ProcessorDescriptor> {
    let mut list = self
      .processors
      .values()
      .map(|item| item.descriptor())
      .collect::<Vec<_>>();

    list.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    list
  }

  pub fn default_registry() -> Self {
    let mut registry = Self::new();
    registry.register(TrimTransparentProcessor::default());
    registry.register(FormatConvertProcessor::default());
    registry.register(CompressProcessor::default());
    registry.register(RepairProcessor::default());
    registry.register(ResolutionTransformProcessor::default());
    registry.register(RenameProcessor::default());
    registry
  }
}
