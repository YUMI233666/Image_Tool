export type ProcessorId =
  | "trim-transparent"
  | "format-convert"
  | "compress"
  | "repair";

export type ItemStatus = "success" | "failed" | "skipped" | "cancelled";

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
  maxConcurrency?: number;
  includeSubdirectories?: boolean;
  writeReport?: boolean;
}

export interface BatchItemReport {
  inputPath: string;
  outputPath?: string;
  status: ItemStatus;
  message: string;
  durationMs: number;
}

export interface BatchJobReport {
  jobId: string;
  processorId: ProcessorId;
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
}
