import type { BatchItemReport, PathImageInfo } from "../lib/types";

interface FileInspectorPanelProps {
  inputPaths: string[];
  outputItems: BatchItemReport[];
  selectedInputPath: string | null;
  selectedOutputPath: string | null;
  inspectedInfo: PathImageInfo | null;
  isInspecting: boolean;
  inspectError: string;
  onSelectInputPath: (path: string) => void;
  onSelectOutputPath: (path: string) => void;
}

function formatBytes(value?: number): string {
  if (value === undefined || Number.isNaN(value)) {
    return "-";
  }

  if (value < 1024) {
    return `${value} B`;
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(2)} KB`;
  }

  if (value < 1024 * 1024 * 1024) {
    return `${(value / (1024 * 1024)).toFixed(2)} MB`;
  }

  return `${(value / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function toFileTypeLabel(info: PathImageInfo): string {
  if (!info.exists) {
    return "不存在";
  }

  if (info.isDirectory) {
    return "目录";
  }

  if (info.isFile) {
    return "文件";
  }

  return "未知";
}

export default function FileInspectorPanel({
  inputPaths,
  outputItems,
  selectedInputPath,
  selectedOutputPath,
  inspectedInfo,
  isInspecting,
  inspectError,
  onSelectInputPath,
  onSelectOutputPath,
}: FileInspectorPanelProps) {
  const outputPaths = outputItems.filter((item) => Boolean(item.outputPath));

  return (
    <section className="panel panel-full-width">
      <h2>文件列表与参数信息</h2>

      <div className="file-lists-grid">
        <div className="file-list-box">
          <h3>添加的文件</h3>
          {inputPaths.length === 0 ? (
            <p className="muted">暂无输入文件，先在上方添加后可选择查看参数。</p>
          ) : (
            <ul className="selectable-list">
              {inputPaths.map((path) => {
                const selected = selectedInputPath === path;
                return (
                  <li key={`input-${path}`}>
                    <button
                      type="button"
                      className={`path-item ${selected ? "is-selected" : ""}`}
                      onClick={() => onSelectInputPath(path)}
                    >
                      <span className="path-item-main">{path}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>

        <div className="file-list-box">
          <h3>输出文件</h3>
          {outputPaths.length === 0 ? (
            <p className="muted">执行处理后会在这里展示输出文件列表。</p>
          ) : (
            <ul className="selectable-list">
              {outputPaths.map((item) => {
                const outputPath = item.outputPath as string;
                const selected = selectedOutputPath === outputPath;

                return (
                  <li key={`output-${outputPath}-${item.durationMs}`}>
                    <button
                      type="button"
                      className={`path-item ${selected ? "is-selected" : ""}`}
                      onClick={() => onSelectOutputPath(outputPath)}
                    >
                      <span className="path-item-main">{outputPath}</span>
                      <span className={`path-item-meta status-${item.status}`}>
                        {item.status}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>

      <div className="image-info-panel">
        <h3>图片参数信息</h3>

        {isInspecting ? <p className="muted">正在更新参数信息...</p> : null}
        {inspectError ? <p className="error-inline">{inspectError}</p> : null}

        {!inspectError && !inspectedInfo && !isInspecting ? (
          <p className="muted">请选择输入文件或输出文件来查看参数信息。</p>
        ) : null}

        {inspectedInfo ? (
          <div className="image-info-grid">
            <div className="info-item full-width">
              <span>路径</span>
              <strong>{inspectedInfo.path}</strong>
            </div>
            <div className="info-item">
              <span>路径类型</span>
              <strong>{toFileTypeLabel(inspectedInfo)}</strong>
            </div>
            <div className="info-item">
              <span>文件大小</span>
              <strong>{formatBytes(inspectedInfo.fileSizeBytes)}</strong>
            </div>
            <div className="info-item">
              <span>图片格式</span>
              <strong>{inspectedInfo.imageFormat ?? "-"}</strong>
            </div>
            <div className="info-item">
              <span>色彩类型</span>
              <strong>{inspectedInfo.colorType ?? "-"}</strong>
            </div>
            <div className="info-item">
              <span>分辨率</span>
              <strong>
                {inspectedInfo.width && inspectedInfo.height
                  ? `${inspectedInfo.width} x ${inspectedInfo.height}`
                  : "-"}
              </strong>
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
