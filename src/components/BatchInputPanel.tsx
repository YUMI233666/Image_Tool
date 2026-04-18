import { open } from "@tauri-apps/api/dialog";

interface BatchInputPanelProps {
  inputPaths: string[];
  outputDir: string;
  includeSubdirectories: boolean;
  maxConcurrency: number;
  isRunning: boolean;
  onInputPathsChange: (paths: string[]) => void;
  onOutputDirChange: (path: string) => void;
  onIncludeSubdirectoriesChange: (value: boolean) => void;
  onMaxConcurrencyChange: (value: number) => void;
}

function normalizeDialogResult(
  value: string | string[] | null,
  defaultValue: string[] = [],
): string[] {
  if (value === null) {
    return defaultValue;
  }

  return Array.isArray(value) ? value : [value];
}

export default function BatchInputPanel({
  inputPaths,
  outputDir,
  includeSubdirectories,
  maxConcurrency,
  isRunning,
  onInputPathsChange,
  onOutputDirChange,
  onIncludeSubdirectoriesChange,
  onMaxConcurrencyChange,
}: BatchInputPanelProps) {
  const pickFiles = async () => {
    const result = await open({
      multiple: true,
      filters: [
        {
          name: "Images",
          extensions: ["png", "jpg", "jpeg", "webp", "bmp"],
        },
      ],
    });

    const selected = normalizeDialogResult(result, inputPaths);
    onInputPathsChange(selected);
  };

  const pickFolder = async () => {
    const result = await open({
      directory: true,
      multiple: false,
    });

    const selected = normalizeDialogResult(result, inputPaths);
    if (selected.length > 0) {
      onInputPathsChange(selected);
    }
  };

  const pickOutputDir = async () => {
    const result = await open({
      directory: true,
      multiple: false,
    });

    const selected = normalizeDialogResult(result);
    if (selected.length > 0) {
      onOutputDirChange(selected[0]);
    }
  };

  return (
    <section className="panel">
      <h2>输入与输出</h2>

      <div className="toolbar">
        <button type="button" onClick={pickFiles} disabled={isRunning}>
          选择图片文件
        </button>
        <button type="button" onClick={pickFolder} disabled={isRunning}>
          选择输入文件夹
        </button>
        <button
          type="button"
          className="ghost"
          onClick={() => onInputPathsChange([])}
          disabled={isRunning || inputPaths.length === 0}
        >
          清空输入
        </button>
      </div>

      <div className="input-list">
        {inputPaths.length === 0 ? (
          <p className="muted">尚未选择输入文件或文件夹。</p>
        ) : (
          <>
            <p className="muted">已选择 {inputPaths.length} 项输入。</p>
            <ul>
              {inputPaths.slice(0, 6).map((path) => (
                <li key={path}>{path}</li>
              ))}
            </ul>
            {inputPaths.length > 6 ? (
              <p className="hint">仅展示前6项，实际会处理全部输入。</p>
            ) : null}
          </>
        )}
      </div>

      <label className="field">
        <span>输出目录</span>
        <div className="inline-actions">
          <input
            value={outputDir}
            onChange={(event) => onOutputDirChange(event.target.value)}
            placeholder="请选择输出目录"
            disabled={isRunning}
          />
          <button type="button" onClick={pickOutputDir} disabled={isRunning}>
            浏览
          </button>
        </div>
      </label>

      <label className="field inline-checkbox">
        <input
          type="checkbox"
          checked={includeSubdirectories}
          onChange={(event) => onIncludeSubdirectoriesChange(event.target.checked)}
          disabled={isRunning}
        />
        <span>递归处理子目录</span>
      </label>

      <label className="field">
        <span>最大并发数</span>
        <input
          type="number"
          min={1}
          max={64}
          value={maxConcurrency}
          onChange={(event) => onMaxConcurrencyChange(Number(event.target.value))}
          disabled={isRunning}
        />
      </label>
    </section>
  );
}
