import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import BatchInputPanel from "./components/BatchInputPanel";
import FileInspectorPanel from "./components/FileInspectorPanel";
import FunctionSelector from "./components/FunctionSelector";
import RenameRulePanel from "./components/RenameRulePanel";
import ResultSummaryPanel from "./components/ResultSummaryPanel";
import TaskQueuePanel from "./components/TaskQueuePanel";
import WorkflowBuilder from "./components/WorkflowBuilder";
import {
  cancelBatchJob,
  getPathImageInfo,
  listProcessors,
  listenBatchComplete,
  listenBatchProgress,
  openPathInSystem,
  previewDiscoveredFiles,
  startBatchJob,
} from "./lib/api/tauri";
import type {
  BatchItemReport,
  PathImageInfo,
  ProcessorDescriptor,
  ProcessorId,
  WorkflowStepRequest,
} from "./lib/types";
import { useTaskStore } from "./store/taskStore";

const fallbackProcessors: ProcessorDescriptor[] = [
  {
    id: "trim-transparent",
    displayName: "裁剪透明边缘",
    enabled: true,
    notes: "裁剪 PNG 的透明边缘到非透明像素区域。",
  },
  {
    id: "format-convert",
    displayName: "图像格式转换",
    enabled: true,
    notes: "支持 PNG/JPG/WEBP 格式互转。",
  },
  {
    id: "compress",
    displayName: "图像压缩",
    enabled: true,
    notes: "支持 JPG/PNG/WEBP 压缩（BMP/TIFF 建议先转换后再压缩）。",
  },
  {
    id: "repair",
    displayName: "图像修复",
    enabled: true,
    notes: "支持自动修复、边缘保留去噪、轻度划痕修复与低分辨率增强（独立锐化强度）。",
  },
  {
    id: "resolution-transform",
    displayName: "变换分辨率",
    enabled: true,
    notes: "支持目标分辨率缩放：目标更小时压缩、目标更大时超分；PNG 可透明居中占位，支持单文件目标覆盖。",
  },
  {
    id: "rename",
    displayName: "批量重命名",
    enabled: true,
    notes: "仅修改输出文件名，不改变图片内容。",
  },
];

const INSPECT_LOADING_DELAY_MS = 180;
const RESOLUTION_MIN_EDGE = 1;
const RESOLUTION_MAX_EDGE = 16384;
const PARAM_MIN_STRENGTH = 1;
const PARAM_MAX_STRENGTH = 100;

type ResolutionFileOverrideMap = Record<
  string,
  { targetWidth?: unknown; targetHeight?: unknown }
>;

function clampInt(value: unknown, min: number, max: number): number {
  if (typeof value !== "number" || Number.isNaN(value)) {
    return min;
  }

  return Math.min(max, Math.max(min, Math.floor(value)));
}

export default function App() {
  const {
    availableProcessors,
    runMode,
    selectedProcessorId,
    workflowSteps,
    activeWorkflowStepId,
    renameConfig,
    inputPaths,
    outputDir,
    includeSubdirectories,
    maxConcurrency,
    paramsByProcessor,
    activeJobId,
    isRunning,
    progress,
    report,
    setAvailableProcessors,
    setRunMode,
    setSelectedProcessorId,
    addWorkflowStep,
    removeWorkflowStep,
    moveWorkflowStep,
    setActiveWorkflowStepId,
    updateWorkflowStepProcessor,
    patchWorkflowStepParams,
    patchRenameConfig,
    setInputPaths,
    setOutputDir,
    setIncludeSubdirectories,
    setMaxConcurrency,
    patchParams,
    beginRun,
    setActiveJobId,
    setProgress,
    finishRun,
    resetReport,
  } = useTaskStore();

  const [uiError, setUiError] = useState<string>("");
  const [selectedInputPath, setSelectedInputPath] = useState<string | null>(null);
  const [selectedOutputPath, setSelectedOutputPath] = useState<string | null>(null);
  const [inspectedInfo, setInspectedInfo] = useState<PathImageInfo | null>(null);
  const [isInspecting, setIsInspecting] = useState(false);
  const [inspectError, setInspectError] = useState("");
  const [overrideCandidatePaths, setOverrideCandidatePaths] = useState<string[]>([]);
  const [isLoadingOverrideCandidates, setIsLoadingOverrideCandidates] = useState(false);
  const [overrideCandidatesError, setOverrideCandidatesError] = useState("");
  const inspectRequestIdRef = useRef(0);
  const inspectLoadingTimerRef = useRef<number | null>(null);
  const inspectCacheRef = useRef<Map<string, PathImageInfo>>(new Map());
  const progressUnlistenRef = useRef<(() => void) | null>(null);
  const completeUnlistenRef = useRef<(() => void) | null>(null);
  const bindingPromiseRef = useRef<Promise<void> | null>(null);

  useEffect(() => {
    let mounted = true;

    listProcessors()
      .then((items) => {
        if (!mounted) {
          return;
        }

        if (items.length === 0) {
          setAvailableProcessors(fallbackProcessors);
          return;
        }

        setAvailableProcessors(items);
      })
      .catch(() => {
        if (mounted) {
          setAvailableProcessors(fallbackProcessors);
        }
      });

    return () => {
      mounted = false;
    };
  }, [setAvailableProcessors]);

  useEffect(() => {
    return () => {
      if (inspectLoadingTimerRef.current !== null) {
        window.clearTimeout(inspectLoadingTimerRef.current);
        inspectLoadingTimerRef.current = null;
      }
    };
  }, []);

  const ensureBatchListenersBound = useCallback(async () => {
    if (progressUnlistenRef.current && completeUnlistenRef.current) {
      return;
    }

    if (bindingPromiseRef.current) {
      await bindingPromiseRef.current;
      return;
    }

    bindingPromiseRef.current = (async () => {
      const unlistenProgress = await listenBatchProgress((payload) => {
        setActiveJobId(payload.jobId);
        setProgress(payload);
      });

      const unlistenComplete = await listenBatchComplete((payload) => {
        finishRun(payload);
      });

      progressUnlistenRef.current = unlistenProgress;
      completeUnlistenRef.current = unlistenComplete;
    })();

    try {
      await bindingPromiseRef.current;
    } finally {
      bindingPromiseRef.current = null;
    }
  }, [finishRun, setActiveJobId, setProgress]);

  useEffect(() => {
    ensureBatchListenersBound().catch(() => undefined);

    return () => {
      if (progressUnlistenRef.current) {
        progressUnlistenRef.current();
        progressUnlistenRef.current = null;
      }

      if (completeUnlistenRef.current) {
        completeUnlistenRef.current();
        completeUnlistenRef.current = null;
      }
    };
  }, [ensureBatchListenersBound]);

  const selectedProcessor = useMemo(() => {
    return availableProcessors.find((item) => item.id === selectedProcessorId);
  }, [availableProcessors, selectedProcessorId]);

  const activeWorkflowStep = useMemo<WorkflowStepRequest | null>(() => {
    if (workflowSteps.length === 0) {
      return null;
    }

    return (
      workflowSteps.find((step) => step.stepId === activeWorkflowStepId) ??
      workflowSteps[0]
    );
  }, [activeWorkflowStepId, workflowSteps]);

  const activeWorkflowStepIndex = useMemo(() => {
    if (!activeWorkflowStepId) {
      return -1;
    }

    return workflowSteps.findIndex((step) => step.stepId === activeWorkflowStepId);
  }, [activeWorkflowStepId, workflowSteps]);

  const parameterEditorProcessorId =
    runMode === "quick"
      ? selectedProcessorId
      : (activeWorkflowStep?.processorId ?? null);

  const outputItems = useMemo<BatchItemReport[]>(() => {
    if (!report) {
      return [];
    }

    return report.items.filter((item) => Boolean(item.outputPath));
  }, [report]);

  const selectedParams =
    runMode === "quick"
      ? paramsByProcessor[selectedProcessorId]
      : (activeWorkflowStep?.params ?? {});

  const patchCurrentParams = useCallback(
    (processorId: ProcessorId, patch: Record<string, unknown>) => {
      if (runMode === "quick") {
        patchParams(processorId, patch);
        return;
      }

      if (!activeWorkflowStep || activeWorkflowStep.processorId !== processorId) {
        return;
      }

      patchWorkflowStepParams(activeWorkflowStep.stepId, patch);
    },
    [activeWorkflowStep, patchParams, patchWorkflowStepParams, runMode],
  );

  useEffect(() => {
    if (parameterEditorProcessorId !== "resolution-transform") {
      setOverrideCandidatePaths([]);
      setIsLoadingOverrideCandidates(false);
      setOverrideCandidatesError("");
      return;
    }

    if (inputPaths.length === 0) {
      setOverrideCandidatePaths([]);
      setIsLoadingOverrideCandidates(false);
      setOverrideCandidatesError("");
      return;
    }

    let cancelled = false;
    setIsLoadingOverrideCandidates(true);
    setOverrideCandidatesError("");

    previewDiscoveredFiles(
      "resolution-transform",
      inputPaths,
      includeSubdirectories,
    )
      .then((paths) => {
        if (cancelled) {
          return;
        }

        setOverrideCandidatePaths(paths);
      })
      .catch((error) => {
        if (cancelled) {
          return;
        }

        const reason =
          error instanceof Error
            ? error.message
            : "加载批处理文件列表失败，已回退为输入路径列表。";
        setOverrideCandidatesError(reason);
        setOverrideCandidatePaths(inputPaths);
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoadingOverrideCandidates(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [includeSubdirectories, inputPaths, parameterEditorProcessorId]);

  const inspectPathInfo = async (path: string) => {
    const requestId = inspectRequestIdRef.current + 1;
    inspectRequestIdRef.current = requestId;

    setInspectError("");

    const cached = inspectCacheRef.current.get(path);
    if (cached) {
      setIsInspecting(false);
      setInspectedInfo(cached);
      return;
    }

    if (inspectLoadingTimerRef.current !== null) {
      window.clearTimeout(inspectLoadingTimerRef.current);
      inspectLoadingTimerRef.current = null;
    }

    inspectLoadingTimerRef.current = window.setTimeout(() => {
      if (inspectRequestIdRef.current === requestId) {
        setIsInspecting(true);
      }
    }, INSPECT_LOADING_DELAY_MS);

    try {
      const info = await getPathImageInfo(path);
      if (inspectRequestIdRef.current !== requestId) {
        return;
      }

      inspectCacheRef.current.set(path, info);
      setInspectedInfo(info);
    } catch (error) {
      if (inspectRequestIdRef.current !== requestId) {
        return;
      }

      const reason =
        error instanceof Error ? error.message : "读取图片参数信息失败。";
      setInspectedInfo(null);
      setInspectError(reason);
    } finally {
      if (inspectRequestIdRef.current === requestId) {
        if (inspectLoadingTimerRef.current !== null) {
          window.clearTimeout(inspectLoadingTimerRef.current);
          inspectLoadingTimerRef.current = null;
        }

        setIsInspecting(false);
      }
    }
  };

  const handleSelectInputPath = (path: string) => {
    if (selectedInputPath === path && selectedOutputPath === null) {
      return;
    }

    setSelectedInputPath(path);
    setSelectedOutputPath(null);
    inspectPathInfo(path).catch(() => undefined);
  };

  const handleSelectOutputPath = (path: string) => {
    if (selectedOutputPath === path && selectedInputPath === null) {
      return;
    }

    setSelectedOutputPath(path);
    setSelectedInputPath(null);
    inspectPathInfo(path).catch(() => undefined);
  };

  useEffect(() => {
    if (!selectedInputPath) {
      return;
    }

    if (!inputPaths.includes(selectedInputPath)) {
      setSelectedInputPath(null);
      setInspectedInfo(null);
      setInspectError("");
    }
  }, [inputPaths, selectedInputPath]);

  useEffect(() => {
    if (!selectedOutputPath) {
      return;
    }

    const stillExists = outputItems.some((item) => item.outputPath === selectedOutputPath);
    if (!stillExists) {
      setSelectedOutputPath(null);
      setInspectedInfo(null);
      setInspectError("");
    }
  }, [outputItems, selectedOutputPath]);

  const start = async () => {
    setUiError("");
    resetReport();

    if (inputPaths.length === 0) {
      setUiError("请先选择输入文件或输入目录。");
      return;
    }

    if (!outputDir.trim()) {
      setUiError("请先选择输出目录。");
      return;
    }

    if (runMode === "quick") {
      if (!selectedProcessor?.enabled) {
        setUiError("当前功能暂不可执行。");
        return;
      }
    } else {
      if (workflowSteps.length === 0) {
        setUiError("工作流模式至少需要一个步骤。");
        return;
      }

      const unavailableStep = workflowSteps.find((step) => {
        const processor = availableProcessors.find(
          (item) => item.id === step.processorId,
        );
        return !processor || !processor.enabled;
      });

      if (unavailableStep) {
        setUiError(`工作流包含不可用步骤：${unavailableStep.processorId}`);
        return;
      }
    }

    try {
      await ensureBatchListenersBound();
    } catch {
      setUiError("任务进度监听初始化失败，请重试。");
      return;
    }

    beginRun();
    setProgress({
      jobId: activeJobId ?? "pending",
      processed: 0,
      total: 0,
      succeeded: 0,
      failed: 0,
      skipped: 0,
      cancelled: 0,
      currentFile: "",
      status: "running",
      message: "任务已启动，等待后端回传进度...",
    });

    const effectiveProcessorId: ProcessorId =
      runMode === "quick"
        ? selectedProcessorId
        : (workflowSteps[0]?.processorId ?? selectedProcessorId);

    const workflowPayload =
      runMode === "workflow"
        ? workflowSteps.map((step) => ({
            stepId: step.stepId,
            processorId: step.processorId,
            params: step.params,
          }))
        : undefined;

    try {
      const nextReport = await startBatchJob({
        processorId: effectiveProcessorId,
        inputPaths,
        outputDir,
        params: runMode === "quick" ? selectedParams : {},
        workflowSteps: workflowPayload,
        renameConfig: renameConfig.enabled ? renameConfig : undefined,
        includeSubdirectories,
        maxConcurrency,
        writeReport: true,
      });

      finishRun(nextReport);
    } catch (error) {
      const reason = error instanceof Error ? error.message : "任务启动失败";
      setUiError(reason);
      finishRun({
        jobId: activeJobId ?? "",
        processorId: runMode === "workflow" ? "workflow" : selectedProcessorId,
        startedAt: new Date().toISOString(),
        finishedAt: new Date().toISOString(),
        total: 0,
        succeeded: 0,
        failed: 0,
        skipped: 0,
        cancelled: 0,
        items: [],
      });
    }
  };

  const cancel = async () => {
    if (!activeJobId) {
      return;
    }

    try {
      await cancelBatchJob(activeJobId);
    } catch {
      setUiError("取消任务失败，请稍后重试。");
    }
  };

  const openOutputDir = async () => {
    if (!outputDir) {
      return;
    }

    try {
      await openPathInSystem(outputDir);
      setUiError("");
    } catch (error) {
      const reason = error instanceof Error ? error.message : "打开输出目录失败。";
      setUiError(reason);
    }
  };

  const openReport = async () => {
    if (!report?.reportPath) {
      return;
    }

    try {
      await openPathInSystem(report.reportPath);
      setUiError("");
    } catch (error) {
      const reason = error instanceof Error ? error.message : "打开报告失败。";
      setUiError(reason);
    }
  };

  const renderParams = () => {
    if (!parameterEditorProcessorId) {
      return <p className="muted">请先在工作流中选择一个步骤。</p>;
    }

    switch (parameterEditorProcessorId) {
      case "trim-transparent":
        return (
          <>
            <label className="field">
              <span>透明阈值 (0-255)</span>
              <input
                type="number"
                min={0}
                max={255}
                disabled={isRunning}
                value={clampInt(selectedParams.alphaThreshold, 0, 255)}
                onChange={(event) =>
                  patchCurrentParams("trim-transparent", {
                    alphaThreshold: clampInt(
                      Number(event.target.value),
                      0,
                      255,
                    ),
                  })
                }
              />
            </label>
            <label className="field">
              <span>保留边距 (px)</span>
              <input
                type="number"
                min={0}
                max={200}
                disabled={isRunning}
                value={clampInt(selectedParams.padding, 0, 200)}
                onChange={(event) =>
                  patchCurrentParams("trim-transparent", {
                    padding: clampInt(Number(event.target.value), 0, 200),
                  })
                }
              />
            </label>
          </>
        );
      case "format-convert":
        return (
          <label className="field">
            <span>目标格式</span>
            <select
              value={String(selectedParams.targetFormat ?? "png")}
              onChange={(event) =>
                patchCurrentParams("format-convert", {
                  targetFormat: event.target.value,
                })
              }
              disabled={isRunning}
            >
              <option value="png">PNG</option>
              <option value="jpg">JPG</option>
              <option value="webp">WEBP</option>
            </select>
          </label>
        );
      case "compress":
        return (
          <>
            <label className="field">
              <span>压缩模式</span>
              <select
                value={String(selectedParams.mode ?? "balanced")}
                onChange={(event) =>
                  patchCurrentParams("compress", {
                    mode: event.target.value,
                  })
                }
                disabled={isRunning}
              >
                <option value="balanced">balanced（平衡）</option>
                <option value="lossy">lossy（有损）</option>
                <option value="lossless">lossless（无损）</option>
              </select>
            </label>
            <label className="field">
              <span>压缩质量 (1-100)</span>
              <input
                type="number"
                min={1}
                max={100}
                disabled={isRunning}
                value={clampInt(selectedParams.quality, 1, 100)}
                onChange={(event) =>
                  patchCurrentParams("compress", {
                    quality: clampInt(Number(event.target.value), 1, 100),
                  })
                }
              />
            </label>
          </>
        );
      case "repair":
      {
        const repairMode = String(selectedParams.mode ?? "auto");
        return (
          <>
            <label className="field">
              <span>修复模式</span>
              <select
                value={repairMode}
                onChange={(event) =>
                  patchCurrentParams("repair", { mode: event.target.value })
                }
                disabled={isRunning}
              >
                <option value="auto">自动</option>
                <option value="denoise">去噪</option>
                <option value="scratch">划痕修复</option>
                <option value="upscale">低分辨率增强</option>
              </select>
            </label>
            <label className="field">
              <span>修复强度 (1-100)</span>
              <input
                type="number"
                min={PARAM_MIN_STRENGTH}
                max={PARAM_MAX_STRENGTH}
                disabled={isRunning}
                value={clampInt(selectedParams.strength, PARAM_MIN_STRENGTH, PARAM_MAX_STRENGTH)}
                onChange={(event) =>
                  patchCurrentParams("repair", {
                    strength: clampInt(
                      Number(event.target.value),
                      PARAM_MIN_STRENGTH,
                      PARAM_MAX_STRENGTH,
                    ),
                  })
                }
              />
            </label>
            {repairMode === "upscale" ? (
              <>
                <label className="field">
                  <span>放大倍数 (2-4)</span>
                  <select
                    value={String(clampInt(selectedParams.upscaleFactor, 2, 4))}
                    onChange={(event) =>
                      patchCurrentParams("repair", {
                        upscaleFactor: clampInt(Number(event.target.value), 2, 4),
                      })
                    }
                    disabled={isRunning}
                  >
                    <option value="2">2x</option>
                    <option value="3">3x</option>
                    <option value="4">4x</option>
                  </select>
                </label>
                <label className="field">
                  <span>超分锐化强度 (1-100)</span>
                  <input
                    type="number"
                    min={PARAM_MIN_STRENGTH}
                    max={PARAM_MAX_STRENGTH}
                    disabled={isRunning}
                    value={clampInt(
                      selectedParams.upscaleSharpness,
                      PARAM_MIN_STRENGTH,
                      PARAM_MAX_STRENGTH,
                    )}
                    onChange={(event) =>
                      patchCurrentParams("repair", {
                        upscaleSharpness: clampInt(
                          Number(event.target.value),
                          PARAM_MIN_STRENGTH,
                          PARAM_MAX_STRENGTH,
                        ),
                      })
                    }
                  />
                </label>
              </>
            ) : null}
          </>
        );
      }
      case "resolution-transform":
      {
        const candidatePaths =
          overrideCandidatePaths.length > 0
            ? overrideCandidatePaths
            : inputPaths;
        const globalTargetWidth = clampInt(
          selectedParams.targetWidth,
          RESOLUTION_MIN_EDGE,
          RESOLUTION_MAX_EDGE,
        );
        const globalTargetHeight = clampInt(
          selectedParams.targetHeight,
          RESOLUTION_MIN_EDGE,
          RESOLUTION_MAX_EDGE,
        );
        const sharpness = clampInt(
          selectedParams.upscaleSharpness,
          PARAM_MIN_STRENGTH,
          PARAM_MAX_STRENGTH,
        );
        const fileOverrides =
          (selectedParams.fileOverrides as ResolutionFileOverrideMap | undefined) ?? {};

        const setFileOverrideEnabled = (path: string, enabled: boolean) => {
          const nextOverrides = { ...fileOverrides };

          if (!enabled) {
            delete nextOverrides[path];
          } else {
            nextOverrides[path] = {
              targetWidth: globalTargetWidth,
              targetHeight: globalTargetHeight,
            };
          }

          patchCurrentParams("resolution-transform", {
            fileOverrides: nextOverrides,
          });
        };

        const patchFileOverride = (
          path: string,
          patch: { targetWidth?: number; targetHeight?: number },
        ) => {
          const current = fileOverrides[path] ?? {
            targetWidth: globalTargetWidth,
            targetHeight: globalTargetHeight,
          };

          patchCurrentParams("resolution-transform", {
            fileOverrides: {
              ...fileOverrides,
              [path]: {
                targetWidth: clampInt(
                  patch.targetWidth ?? current.targetWidth,
                  RESOLUTION_MIN_EDGE,
                  RESOLUTION_MAX_EDGE,
                ),
                targetHeight: clampInt(
                  patch.targetHeight ?? current.targetHeight,
                  RESOLUTION_MIN_EDGE,
                  RESOLUTION_MAX_EDGE,
                ),
              },
            },
          });
        };

        return (
          <>
                  <label className="field">
                    <span>目标宽度 (1-16384)</span>
                    <input
                      type="number"
                      min={RESOLUTION_MIN_EDGE}
                      max={RESOLUTION_MAX_EDGE}
                      disabled={isRunning}
                      value={globalTargetWidth}
                      onChange={(event) =>
                        patchCurrentParams("resolution-transform", {
                          targetWidth: clampInt(
                            Number(event.target.value),
                            RESOLUTION_MIN_EDGE,
                            RESOLUTION_MAX_EDGE,
                          ),
                        })
                      }
                    />
                  </label>
                  <label className="field">
                    <span>目标高度 (1-16384)</span>
                    <input
                      type="number"
                      min={RESOLUTION_MIN_EDGE}
                      max={RESOLUTION_MAX_EDGE}
                      disabled={isRunning}
                      value={globalTargetHeight}
                      onChange={(event) =>
                        patchCurrentParams("resolution-transform", {
                          targetHeight: clampInt(
                            Number(event.target.value),
                            RESOLUTION_MIN_EDGE,
                            RESOLUTION_MAX_EDGE,
                          ),
                        })
                      }
                    />
                  </label>
                  <label className="field">
                    <span>超分锐化强度 (1-100)</span>
                    <input
                      type="number"
                      min={PARAM_MIN_STRENGTH}
                      max={PARAM_MAX_STRENGTH}
                      disabled={isRunning}
                      value={sharpness}
                      onChange={(event) =>
                        patchCurrentParams("resolution-transform", {
                          upscaleSharpness: clampInt(
                            Number(event.target.value),
                            PARAM_MIN_STRENGTH,
                            PARAM_MAX_STRENGTH,
                          ),
                        })
                      }
                    />
                  </label>
                  <p className="hint">
                    PNG：当目标比例与图像主体比例不同，会透明居中填充到目标分辨率。JPG/WEBP：始终保持原图比例并适配到目标框内。
                  </p>

                  <section className="input-list resolution-override-list">
                    <h3>单文件目标分辨率（可选）</h3>
                    {isLoadingOverrideCandidates ? (
                      <p className="muted">正在加载本次批处理图片列表...</p>
                    ) : null}
                    {overrideCandidatesError ? (
                      <p className="error-inline">{overrideCandidatesError}</p>
                    ) : null}
                    {candidatePaths.length === 0 ? (
                      <p className="muted">先选择输入文件后可为每个文件单独设置目标分辨率。</p>
                    ) : (
                      <ul className="resolution-override-items">
                        {candidatePaths.map((path) => {
                          const override = fileOverrides[path];
                          const enabled = Boolean(override);
                          const targetWidth = clampInt(
                            override?.targetWidth ?? globalTargetWidth,
                            RESOLUTION_MIN_EDGE,
                            RESOLUTION_MAX_EDGE,
                          );
                          const targetHeight = clampInt(
                            override?.targetHeight ?? globalTargetHeight,
                            RESOLUTION_MIN_EDGE,
                            RESOLUTION_MAX_EDGE,
                          );

                          return (
                            <li key={path} className="resolution-override-item">
                              <label className="inline-checkbox resolution-override-toggle">
                                <input
                                  type="checkbox"
                                  checked={enabled}
                                  disabled={isRunning}
                                  onChange={(event) =>
                                    setFileOverrideEnabled(path, event.target.checked)
                                  }
                                />
                                <span className="resolution-override-path">{path}</span>
                              </label>
                              <div className="resolution-override-controls">
                                <label className="field">
                                  <span>目标宽度</span>
                                  <input
                                    type="number"
                                    min={RESOLUTION_MIN_EDGE}
                                    max={RESOLUTION_MAX_EDGE}
                                    disabled={isRunning || !enabled}
                                    value={targetWidth}
                                    onChange={(event) =>
                                      patchFileOverride(path, {
                                        targetWidth: clampInt(
                                          Number(event.target.value),
                                          RESOLUTION_MIN_EDGE,
                                          RESOLUTION_MAX_EDGE,
                                        ),
                                      })
                                    }
                                  />
                                </label>
                                <label className="field">
                                  <span>目标高度</span>
                                  <input
                                    type="number"
                                    min={RESOLUTION_MIN_EDGE}
                                    max={RESOLUTION_MAX_EDGE}
                                    disabled={isRunning || !enabled}
                                    value={targetHeight}
                                    onChange={(event) =>
                                      patchFileOverride(path, {
                                        targetHeight: clampInt(
                                          Number(event.target.value),
                                          RESOLUTION_MIN_EDGE,
                                          RESOLUTION_MAX_EDGE,
                                        ),
                                      })
                                    }
                                  />
                                </label>
                              </div>
                            </li>
                          );
                        })}
                      </ul>
                    )}
                  </section>
          </>
        );
      }
      case "rename":
        return (
          <p className="muted">
            重命名规则请在“批量重命名”面板中配置。
          </p>
        );
      default:
        return <p className="muted">该功能暂未定义参数。</p>;
    }
  };

  return (
    <main className="app-shell">
      <header className="hero">
        <div>
          <h1>Art Tool</h1>
          <p>
            批量图像处理桌面工具。支持快捷模式与工作流模式，现可进行步骤编排执行与批量重命名（自定义/模板）。
          </p>
        </div>
        <div className="hero-actions">
          <button type="button" onClick={start} disabled={isRunning}>
            开始处理
          </button>
        </div>
      </header>

      {uiError ? <div className="error-banner">{uiError}</div> : null}

      <section className="layout-grid">
        <section className="panel">
          <h2>运行模式</h2>
          <label className="field">
            <span>处理模式</span>
            <select
              value={runMode}
              onChange={(event) => setRunMode(event.target.value as "quick" | "workflow")}
              disabled={isRunning}
            >
              <option value="quick">快捷模式（单功能）</option>
              <option value="workflow">工作流模式（多步骤）</option>
            </select>
          </label>
          <p className="hint">
            快捷模式适合单一任务，工作流模式可按顺序串联多个处理步骤。
          </p>
        </section>

        {runMode === "quick" ? (
          <FunctionSelector
            processors={availableProcessors.length ? availableProcessors : fallbackProcessors}
            selectedProcessorId={selectedProcessorId}
            onSelect={(id) => setSelectedProcessorId(id as ProcessorId)}
          />
        ) : (
          <WorkflowBuilder
            processors={availableProcessors.length ? availableProcessors : fallbackProcessors}
            steps={workflowSteps}
            activeStepId={activeWorkflowStep?.stepId ?? null}
            isRunning={isRunning}
            onSelectStep={setActiveWorkflowStepId}
            onAddStep={addWorkflowStep}
            onRemoveStep={removeWorkflowStep}
            onMoveStep={moveWorkflowStep}
            onChangeStepProcessor={updateWorkflowStepProcessor}
          />
        )}

        <section className="panel">
          <h2>
            参数设置
            {runMode === "workflow" && activeWorkflowStepIndex >= 0
              ? `（步骤 ${activeWorkflowStepIndex + 1}）`
              : ""}
          </h2>
          {renderParams()}
        </section>

        <RenameRulePanel
          config={renameConfig}
          isRunning={isRunning}
          onChange={patchRenameConfig}
        />

        <BatchInputPanel
          inputPaths={inputPaths}
          outputDir={outputDir}
          includeSubdirectories={includeSubdirectories}
          maxConcurrency={maxConcurrency}
          isRunning={isRunning}
          onInputPathsChange={setInputPaths}
          onOutputDirChange={setOutputDir}
          onIncludeSubdirectoriesChange={setIncludeSubdirectories}
          onMaxConcurrencyChange={setMaxConcurrency}
        />

        <FileInspectorPanel
          inputPaths={inputPaths}
          outputItems={outputItems}
          selectedInputPath={selectedInputPath}
          selectedOutputPath={selectedOutputPath}
          inspectedInfo={inspectedInfo}
          isInspecting={isInspecting}
          inspectError={inspectError}
          onSelectInputPath={handleSelectInputPath}
          onSelectOutputPath={handleSelectOutputPath}
        />

        <TaskQueuePanel
          isRunning={isRunning}
          progress={progress}
          onCancel={cancel}
          canCancel={Boolean(isRunning && activeJobId)}
        />

        <ResultSummaryPanel
          report={report}
          onOpenOutputDir={openOutputDir}
          onOpenReport={openReport}
        />
      </section>
    </main>
  );
}
