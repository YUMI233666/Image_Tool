export type ProcessorId =
  | "trim-transparent"
  | "format-convert"
  | "compress"
  | "repair"
  | "resolution-transform"
  | "rename";

export type RunMode = "quick" | "workflow";

export type RenameMode = "custom" | "template";

export type ItemStatus = "success" | "failed" | "skipped" | "cancelled" | "running";

export interface ProcessorDescriptor {
  id: ProcessorId;
  displayName: string;
  enabled: boolean;
  notes: string;
}

export interface StartBatchJobRequest {
  processorId: ProcessorId;
  inputPaths: string[];
  outputDir: string;
  params: Record<string, unknown>;
  workflowSteps?: WorkflowStepRequest[];
  renameConfig?: RenameConfig;
  maxConcurrency?: number;
  includeSubdirectories?: boolean;
  writeReport?: boolean;
}

export interface WorkflowStepRequest {
  stepId: string;
  processorId: ProcessorId;
  params: Record<string, unknown>;
}

export interface RenameConfig {
  enabled: boolean;
  mode: RenameMode;
  customName?: string;
  template?: string;
  startIndex?: number;
  indexPadding?: number;
}

export interface BatchStepReport {
  stepIndex: number;
  stepTotal: number;
  processorId: string;
  status: ItemStatus;
  message: string;
  outputPath?: string;
  durationMs: number;
}

export interface BatchItemReport {
  inputPath: string;
  outputPath?: string;
  status: ItemStatus;
  message: string;
  durationMs: number;
  steps?: BatchStepReport[];
}

export interface BatchJobReport {
  jobId: string;
  processorId: ProcessorId | "workflow";
  startedAt: string;
  finishedAt: string;
  total: number;
  succeeded: number;
  failed: number;
  skipped: number;
  cancelled: number;
  reportPath?: string;
  items: BatchItemReport[];
}

export interface BatchProgressPayload {
  jobId: string;
  processed: number;
  total: number;
  succeeded: number;
  failed: number;
  skipped: number;
  cancelled: number;
  currentFile: string;
  status: ItemStatus;
  message: string;
  currentStepProcessorId?: string;
  currentStepIndex?: number;
  currentStepTotal?: number;
  currentStepMessage?: string;
}

export interface PathImageInfo {
  path: string;
  exists: boolean;
  isFile: boolean;
  isDirectory: boolean;
  fileSizeBytes?: number;
  width?: number;
  height?: number;
  imageFormat?: string;
  colorType?: string;
  message?: string;
}
