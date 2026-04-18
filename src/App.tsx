import { useEffect, useMemo, useState } from "react";
import BatchInputPanel from "./components/BatchInputPanel";
import FunctionSelector from "./components/FunctionSelector";
import ResultSummaryPanel from "./components/ResultSummaryPanel";
import TaskQueuePanel from "./components/TaskQueuePanel";
import {
  cancelBatchJob,
  listProcessors,
  listenBatchComplete,
  listenBatchProgress,
  openPathInSystem,
  startBatchJob,
} from "./lib/api/tauri";
import type { ProcessorDescriptor, ProcessorId } from "./lib/types";
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
    notes: "支持常见图像格式互转。",
  },
  {
    id: "compress",
    displayName: "图像压缩",
    enabled: false,
    notes: "扩展预留：用于后续有损/无损压缩策略。",
  },
  {
    id: "repair",
    displayName: "图像修复",
    enabled: false,
    notes: "扩展预留：用于后续去噪、补全、修复。",
  },
];

function clampInt(value: unknown, min: number, max: number): number {
  if (typeof value !== "number" || Number.isNaN(value)) {
    return min;
  }

  return Math.min(max, Math.max(min, Math.floor(value)));
}

export default function App() {
  const {
    availableProcessors,
    selectedProcessorId,
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
    setSelectedProcessorId,
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
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    const bindEvents = async () => {
      unlistenProgress = await listenBatchProgress((payload) => {
        setActiveJobId(payload.jobId);
        setProgress(payload);
      });

      unlistenComplete = await listenBatchComplete((payload) => {
        finishRun(payload);
      });
    };

    bindEvents().catch(() => undefined);

    return () => {
      if (unlistenProgress) {
        unlistenProgress();
      }

      if (unlistenComplete) {
        unlistenComplete();
      }
    };
  }, [finishRun, setActiveJobId, setProgress]);

  const selectedProcessor = useMemo(() => {
    return availableProcessors.find((item) => item.id === selectedProcessorId);
  }, [availableProcessors, selectedProcessorId]);

  const selectedParams = paramsByProcessor[selectedProcessorId];

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

    if (!selectedProcessor?.enabled) {
      setUiError("当前功能处于预留状态，暂不可执行。");
      return;
    }

    beginRun();

    try {
      const nextReport = await startBatchJob({
        processorId: selectedProcessorId,
        inputPaths,
        outputDir,
        params: selectedParams,
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
        processorId: selectedProcessorId,
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
    switch (selectedProcessorId) {
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
                  patchParams("trim-transparent", {
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
                  patchParams("trim-transparent", {
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
                patchParams("format-convert", {
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
          <label className="field">
            <span>压缩质量 (1-100)</span>
            <input
              type="number"
              min={1}
              max={100}
              disabled={isRunning}
              value={clampInt(selectedParams.quality, 1, 100)}
              onChange={(event) =>
                patchParams("compress", {
                  quality: clampInt(Number(event.target.value), 1, 100),
                })
              }
            />
          </label>
        );
      case "repair":
        return (
          <>
            <label className="field">
              <span>修复模式</span>
              <select
                value={String(selectedParams.mode ?? "auto")}
                onChange={(event) =>
                  patchParams("repair", { mode: event.target.value })
                }
                disabled={isRunning}
              >
                <option value="auto">自动</option>
                <option value="denoise">去噪</option>
                <option value="scratch">划痕修复</option>
              </select>
            </label>
            <label className="field">
              <span>修复强度 (1-100)</span>
              <input
                type="number"
                min={1}
                max={100}
                disabled={isRunning}
                value={clampInt(selectedParams.strength, 1, 100)}
                onChange={(event) =>
                  patchParams("repair", {
                    strength: clampInt(Number(event.target.value), 1, 100),
                  })
                }
              />
            </label>
          </>
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
            批量图像处理桌面工具。当前可用功能：PNG 透明边缘裁剪；已预留格式转换、压缩、修复扩展位。
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
        <FunctionSelector
          processors={availableProcessors.length ? availableProcessors : fallbackProcessors}
          selectedProcessorId={selectedProcessorId}
          onSelect={(id) => setSelectedProcessorId(id as ProcessorId)}
        />

        <section className="panel">
          <h2>参数设置</h2>
          {renderParams()}
        </section>

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
