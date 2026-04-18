import { create } from "zustand";
import type {
  BatchJobReport,
  BatchProgressPayload,
  ProcessorDescriptor,
  ProcessorId,
} from "../lib/types";

export interface TaskStore {
  availableProcessors: ProcessorDescriptor[];
  selectedProcessorId: ProcessorId;
  inputPaths: string[];
  outputDir: string;
  includeSubdirectories: boolean;
  maxConcurrency: number;
  paramsByProcessor: Record<ProcessorId, Record<string, unknown>>;
  activeJobId: string | null;
  isRunning: boolean;
  progress: BatchProgressPayload | null;
  report: BatchJobReport | null;
  setAvailableProcessors: (items: ProcessorDescriptor[]) => void;
  setSelectedProcessorId: (id: ProcessorId) => void;
  setInputPaths: (items: string[]) => void;
  setOutputDir: (dir: string) => void;
  setIncludeSubdirectories: (enabled: boolean) => void;
  setMaxConcurrency: (value: number) => void;
  patchParams: (id: ProcessorId, patch: Record<string, unknown>) => void;
  beginRun: () => void;
  setActiveJobId: (jobId: string | null) => void;
  setProgress: (progress: BatchProgressPayload | null) => void;
  finishRun: (report: BatchJobReport) => void;
  resetReport: () => void;
}

const defaultParams: Record<ProcessorId, Record<string, unknown>> = {
  "trim-transparent": { alphaThreshold: 0, padding: 0 },
  "format-convert": { targetFormat: "png" },
  compress: { quality: 65, mode: "lossy" },
  repair: { mode: "auto", strength: 50, upscaleFactor: 2, upscaleSharpness: 70 },
  "resolution-transform": {
    targetWidth: 1920,
    targetHeight: 1080,
    upscaleSharpness: 70,
    fileOverrides: {},
  },
};

export const useTaskStore = create<TaskStore>((set) => ({
  availableProcessors: [],
  selectedProcessorId: "trim-transparent",
  inputPaths: [],
  outputDir: "",
  includeSubdirectories: false,
  maxConcurrency: 4,
  paramsByProcessor: defaultParams,
  activeJobId: null,
  isRunning: false,
  progress: null,
  report: null,

  setAvailableProcessors: (items) => set({ availableProcessors: items }),

  setSelectedProcessorId: (id) => set({ selectedProcessorId: id }),

  setInputPaths: (items) => set({ inputPaths: items }),

  setOutputDir: (dir) => set({ outputDir: dir }),

  setIncludeSubdirectories: (enabled) =>
    set({ includeSubdirectories: enabled }),

  setMaxConcurrency: (value) =>
    set({ maxConcurrency: Math.max(1, Math.floor(value)) }),

  patchParams: (id, patch) =>
    set((state) => ({
      paramsByProcessor: {
        ...state.paramsByProcessor,
        [id]: {
          ...state.paramsByProcessor[id],
          ...patch,
        },
      },
    })),

  beginRun: () => set({ isRunning: true, report: null, progress: null }),

  setActiveJobId: (jobId) => set({ activeJobId: jobId }),

  setProgress: (progress) => set({ progress }),

  finishRun: (report) =>
    set({
      report,
      isRunning: false,
      activeJobId: null,
      progress: null,
    }),

  resetReport: () => set({ report: null }),
}));
