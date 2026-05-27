import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { readBinaryFile } from "@tauri-apps/api/fs";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { BatchItemReport, ItemStatus, PathImageInfo } from "../lib/types";

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

interface FileListItem {
  path: string;
  status?: ItemStatus;
}

function toFileSrc(path: string): string {
  return convertFileSrc(path.replace(/\\/g, "/"));
}

function createImageBlobUrl(bytes: Uint8Array): string {
  const view = new Uint8Array(bytes);
  const blob = new Blob([view], { type: "image/*" });
  return URL.createObjectURL(blob);
}

interface VirtualizedFileListProps {
  items: FileListItem[];
  selectedPath: string | null;
  emptyLabel: string;
  onSelect: (path: string) => void;
  onPreview: (path: string) => void;
  onThumbError: (path: string) => void;
  thumbErrors: Record<string, boolean>;
  thumbFallbacks: Record<string, string>;
}

function VirtualizedFileList({
  items,
  selectedPath,
  emptyLabel,
  onSelect,
  onPreview,
  onThumbError,
  thumbErrors,
  thumbFallbacks,
}: VirtualizedFileListProps) {
  const parentRef = useRef<HTMLUListElement | null>(null);
  const rowVirtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 120,
    overscan: 6,
  });

  if (items.length === 0) {
    return <p className="muted">{emptyLabel}</p>;
  }

  return (
    <ul ref={parentRef} className="selectable-list selectable-list-virtual">
      <li className="selectable-list-spacer" style={{ height: rowVirtualizer.getTotalSize() }} />
      {rowVirtualizer.getVirtualItems().map((virtualRow) => {
        const item = items[virtualRow.index];
        const selected = selectedPath === item.path;
        const fallbackUrl = thumbFallbacks[item.path];
        const thumbUrl = fallbackUrl ?? toFileSrc(item.path);
        const hasError = Boolean(thumbErrors[item.path] && !fallbackUrl);

        return (
          <li
            key={`${item.path}-${virtualRow.index}`}
            className="selectable-list-row"
            ref={rowVirtualizer.measureElement}
            data-index={virtualRow.index}
            style={{ transform: `translateY(${virtualRow.start}px)` }}
          >
            <button
              type="button"
              className={`path-item ${selected ? "is-selected" : ""}`}
              onClick={() => onSelect(item.path)}
            >
              <span className="path-item-thumb">
                {hasError ? (
                  <span className="thumb-placeholder">无法预览</span>
                ) : (
                  <img
                    src={thumbUrl}
                    alt="thumbnail"
                    loading="lazy"
                    onClick={(event) => {
                      event.stopPropagation();
                      onPreview(item.path);
                    }}
                    onError={() => onThumbError(item.path)}
                  />
                )}
              </span>
              <span className="path-item-main">{item.path}</span>
              {item.status ? (
                <span className={`path-item-meta status-${item.status}`}>{item.status}</span>
              ) : null}
            </button>
          </li>
        );
      })}
    </ul>
  );
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
  const [thumbErrors, setThumbErrors] = useState<Record<string, boolean>>({});
  const [thumbFallbacks, setThumbFallbacks] = useState<Record<string, string>>({});
  const [previewPath, setPreviewPath] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState(false);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);

  const outputPaths = outputItems.filter((item) => Boolean(item.outputPath));
  const inputItems = useMemo<FileListItem[]>(
    () => inputPaths.map((path) => ({ path })),
    [inputPaths],
  );
  const outputList = useMemo<FileListItem[]>(
    () =>
      outputPaths.map((item) => ({
        path: item.outputPath as string,
        status: item.status as ItemStatus,
      })),
    [outputPaths],
  );

  const handleThumbError = useCallback(
    async (path: string) => {
      if (thumbFallbacks[path]) {
        return;
      }

      try {
        const bytes = await readBinaryFile(path);
        const url = createImageBlobUrl(bytes);
        setThumbFallbacks((prev) => ({ ...prev, [path]: url }));
        setThumbErrors((prev) => {
          const next = { ...prev };
          delete next[path];
          return next;
        });
      } catch {
        setThumbErrors((prev) => ({ ...prev, [path]: true }));
      }
    },
    [thumbFallbacks],
  );

  const openPreview = useCallback((path: string) => {
    setPreviewPath(path);
    setPreviewError(false);
    setPreviewSrc(toFileSrc(path));
  }, []);

  const closePreview = useCallback(() => {
    setPreviewPath(null);
    setPreviewError(false);
    setPreviewSrc(null);
  }, []);

  useEffect(() => {
    return () => {
      Object.values(thumbFallbacks).forEach((url) => URL.revokeObjectURL(url));
      if (previewSrc && previewSrc.startsWith("blob:")) {
        URL.revokeObjectURL(previewSrc);
      }
    };
  }, [previewSrc, thumbFallbacks]);

  return (
    <section className="panel panel-full-width">
      <h2>文件列表与参数信息</h2>

      <div className="file-lists-grid">
        <div className="file-list-box">
          <h3>添加的文件</h3>
          <VirtualizedFileList
            items={inputItems}
            selectedPath={selectedInputPath}
            emptyLabel="暂无输入文件，先在上方添加后可选择查看参数。"
            onSelect={onSelectInputPath}
            onPreview={openPreview}
            onThumbError={handleThumbError}
            thumbErrors={thumbErrors}
            thumbFallbacks={thumbFallbacks}
          />
        </div>

        <div className="file-list-box">
          <h3>输出文件</h3>
          <VirtualizedFileList
            items={outputList}
            selectedPath={selectedOutputPath}
            emptyLabel="执行处理后会在这里展示输出文件列表。"
            onSelect={onSelectOutputPath}
            onPreview={openPreview}
            onThumbError={handleThumbError}
            thumbErrors={thumbErrors}
            thumbFallbacks={thumbFallbacks}
          />
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

      {previewPath ? (
        <div className="lightbox-overlay" role="dialog" onClick={closePreview}>
          <div className="lightbox-content" onClick={(event) => event.stopPropagation()}>
            <div className="lightbox-header">
              <span className="lightbox-title">预览</span>
              <button type="button" className="ghost" onClick={closePreview}>
                关闭
              </button>
            </div>
            <div className="lightbox-body">
              {previewError || !previewSrc ? (
                <div className="thumb-placeholder">无法预览</div>
              ) : (
                <img
                  src={previewSrc}
                  alt={previewPath}
                  onError={async () => {
                    if (!previewPath) {
                      setPreviewError(true);
                      return;
                    }

                    try {
                      const bytes = await readBinaryFile(previewPath);
                      const url = createImageBlobUrl(bytes);
                      setPreviewSrc(url);
                      setPreviewError(false);
                    } catch {
                      setPreviewError(true);
                    }
                  }}
                />
              )}
            </div>
            <p className="lightbox-caption">{previewPath}</p>
          </div>
        </div>
      ) : null}
    </section>
  );
}
