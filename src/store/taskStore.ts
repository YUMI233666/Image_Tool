import { create } from "zustand";
import type {
  BatchJobReport,
  BatchProgressPayload,
  ProcessorDescriptor,
  ProcessorId,
  RenameConfig,
  RunMode,
  WorkflowStepRequest,
} from "../lib/types";

type MoveDirection = "up" | "down";

function createStepId(): string {
  return `step-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
}

function cloneDefaultParams(processorId: ProcessorId): Record<string, unknown> {
  return { ...defaultParams[processorId] };
}

export interface TaskStore {
  availableProcessors: ProcessorDescriptor[];
  runMode: RunMode;
  selectedProcessorId: ProcessorId;
  workflowSteps: WorkflowStepRequest[];
  activeWorkflowStepId: string | null;
  renameConfig: RenameConfig;
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
  setRunMode: (mode: RunMode) => void;
  setSelectedProcessorId: (id: ProcessorId) => void;
  addWorkflowStep: (processorId: ProcessorId) => void;
  removeWorkflowStep: (stepId: string) => void;
  moveWorkflowStep: (stepId: string, direction: MoveDirection) => void;
  setActiveWorkflowStepId: (stepId: string | null) => void;
  updateWorkflowStepProcessor: (stepId: string, processorId: ProcessorId) => void;
  patchWorkflowStepParams: (stepId: string, patch: Record<string, unknown>) => void;
  patchRenameConfig: (patch: Partial<RenameConfig>) => void;
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
  rename: {},
};

const defaultRenameConfig: RenameConfig = {
  enabled: false,
  mode: "custom",
  customName: "",
  template: "{name}_{index}",
  startIndex: 1,
  indexPadding: 0,
};

const initialWorkflowStep: WorkflowStepRequest = {
  stepId: createStepId(),
  processorId: "trim-transparent",
  params: cloneDefaultParams("trim-transparent"),
};

export const useTaskStore = create<TaskStore>((set) => ({
  availableProcessors: [],
  runMode: "quick",
  selectedProcessorId: "trim-transparent",
  workflowSteps: [initialWorkflowStep],
  activeWorkflowStepId: initialWorkflowStep.stepId,
  renameConfig: defaultRenameConfig,
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

  setRunMode: (mode) =>
    set((state) => {
      if (mode === "workflow" && state.workflowSteps.length === 0) {
        const step: WorkflowStepRequest = {
          stepId: createStepId(),
          processorId: state.selectedProcessorId,
          params: cloneDefaultParams(state.selectedProcessorId),
        };

        return {
          runMode: mode,
          workflowSteps: [step],
          activeWorkflowStepId: step.stepId,
        };
      }

      return { runMode: mode };
    }),

  setSelectedProcessorId: (id) => set({ selectedProcessorId: id }),

  addWorkflowStep: (processorId) =>
    set((state) => {
      const step: WorkflowStepRequest = {
        stepId: createStepId(),
        processorId,
        params: cloneDefaultParams(processorId),
      };

      return {
        workflowSteps: [...state.workflowSteps, step],
        activeWorkflowStepId: step.stepId,
      };
    }),

  removeWorkflowStep: (stepId) =>
    set((state) => {
      const next = state.workflowSteps.filter((step) => step.stepId !== stepId);
      if (next.length === 0) {
        return {
          workflowSteps: [],
          activeWorkflowStepId: null,
        };
      }

      const activeStillExists = next.some(
        (step) => step.stepId === state.activeWorkflowStepId,
      );

      return {
        workflowSteps: next,
        activeWorkflowStepId: activeStillExists
          ? state.activeWorkflowStepId
          : next[0].stepId,
      };
    }),

  moveWorkflowStep: (stepId, direction) =>
    set((state) => {
      const index = state.workflowSteps.findIndex((step) => step.stepId === stepId);
      if (index < 0) {
        return {};
      }

      const targetIndex = direction === "up" ? index - 1 : index + 1;
      if (targetIndex < 0 || targetIndex >= state.workflowSteps.length) {
        return {};
      }

      const next = [...state.workflowSteps];
      const [item] = next.splice(index, 1);
      next.splice(targetIndex, 0, item);

      return {
        workflowSteps: next,
      };
    }),

  setActiveWorkflowStepId: (stepId) => set({ activeWorkflowStepId: stepId }),

  updateWorkflowStepProcessor: (stepId, processorId) =>
    set((state) => ({
      workflowSteps: state.workflowSteps.map((step) =>
        step.stepId === stepId
          ? {
              ...step,
              processorId,
              params: cloneDefaultParams(processorId),
            }
          : step,
      ),
    })),

  patchWorkflowStepParams: (stepId, patch) =>
    set((state) => ({
      workflowSteps: state.workflowSteps.map((step) =>
        step.stepId === stepId
          ? {
              ...step,
              params: {
                ...step.params,
                ...patch,
              },
            }
          : step,
      ),
    })),

  patchRenameConfig: (patch) =>
    set((state) => ({
      renameConfig: {
        ...state.renameConfig,
        ...patch,
      },
    })),

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
