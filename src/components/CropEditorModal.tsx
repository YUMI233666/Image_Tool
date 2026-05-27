import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import ReactCrop, { type Crop, type PixelCrop } from "react-image-crop";
import { convertFileSrc } from "@tauri-apps/api/tauri";
import { getPathImageInfo } from "../lib/api/tauri";
import type { CropPercentRect, CropRect, ManualCropParams } from "../lib/types";
import "react-image-crop/dist/ReactCrop.css";

interface CropEditorModalProps {
  isOpen: boolean;
  isRunning: boolean;
  paths: string[];
  activePath: string | null;
  params: ManualCropParams;
  onPatchParams: (patch: Partial<ManualCropParams>) => void;
  onSelectPath: (path: string) => void;
  onClose: () => void;
}

interface ImageSize {
  width: number;
  height: number;
}

const DEFAULT_CROP: Crop = {
  unit: "%",
  x: 10,
  y: 10,
  width: 80,
  height: 80,
};

function toFileSrc(path: string): string {
  return convertFileSrc(path.replace(/\\/g, "/"));
}

function parseAspectRatio(value: string): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) {
    return undefined;
  }

  if (trimmed.includes(":")) {
    const [rawW, rawH] = trimmed.split(":");
    const w = Number(rawW);
    const h = Number(rawH);
    if (Number.isFinite(w) && Number.isFinite(h) && w > 0 && h > 0) {
      return w / h;
    }
    return undefined;
  }

  const numeric = Number(trimmed);
  if (Number.isFinite(numeric) && numeric > 0) {
    return numeric;
  }

  return undefined;
}

function toPercentCrop(rect: CropRect, size: ImageSize): Crop {
  return {
    unit: "%",
    x: (rect.x / size.width) * 100,
    y: (rect.y / size.height) * 100,
    width: (rect.width / size.width) * 100,
    height: (rect.height / size.height) * 100,
  };
}

function fromPercentRect(rect: CropPercentRect): Crop {
  return {
    unit: "%",
    x: rect.x * 100,
    y: rect.y * 100,
    width: rect.width * 100,
    height: rect.height * 100,
  };
}

function toPercentRect(crop: Crop): CropPercentRect {
  return {
    x: (crop.x ?? 0) / 100,
    y: (crop.y ?? 0) / 100,
    width: (crop.width ?? 0) / 100,
    height: (crop.height ?? 0) / 100,
  };
}

function toRect(pixelCrop: PixelCrop | null): CropRect | null {
  if (!pixelCrop) {
    return null;
  }

  return {
    x: Math.round(pixelCrop.x),
    y: Math.round(pixelCrop.y),
    width: Math.round(pixelCrop.width),
    height: Math.round(pixelCrop.height),
  };
}

function normalizeCropToAspect(crop: Crop, aspect: number | undefined, size: ImageSize): Crop {
  if (!aspect || !crop.width || !crop.height) {
    return crop;
  }

  const widthPx = (crop.width / 100) * size.width;
  const heightPx = (crop.height / 100) * size.height;
  const xPx = ((crop.x ?? 0) / 100) * size.width;
  const yPx = ((crop.y ?? 0) / 100) * size.height;

  let targetW = widthPx;
  let targetH = targetW / aspect;

  if (!Number.isFinite(targetH) || targetH <= 0) {
    targetH = heightPx;
    targetW = targetH * aspect;
  }

  if (targetH > size.height) {
    targetH = size.height;
    targetW = targetH * aspect;
  }

  if (targetW > size.width) {
    targetW = size.width;
    targetH = targetW / aspect;
  }

  const centerX = xPx + widthPx / 2;
  const centerY = yPx + heightPx / 2;

  let nextX = centerX - targetW / 2;
  let nextY = centerY - targetH / 2;

  nextX = Math.max(0, Math.min(nextX, size.width - targetW));
  nextY = Math.max(0, Math.min(nextY, size.height - targetH));

  return {
    unit: "%",
    x: (nextX / size.width) * 100,
    y: (nextY / size.height) * 100,
    width: (targetW / size.width) * 100,
    height: (targetH / size.height) * 100,
  };
}

export default function CropEditorModal({
  isOpen,
  isRunning,
  paths,
  activePath,
  params,
  onPatchParams,
  onSelectPath,
  onClose,
}: CropEditorModalProps) {
  const [crop, setCrop] = useState<Crop>(DEFAULT_CROP);
  const [pixelCrop, setPixelCrop] = useState<PixelCrop | null>(null);
  const [imageSize, setImageSize] = useState<ImageSize | null>(null);
  const [loadError, setLoadError] = useState("");
  const [actionError, setActionError] = useState("");
  const [isValidating, setIsValidating] = useState(false);

  const aspect = useMemo(
    () => parseAspectRatio(params.aspectRatio ?? ""),
    [params.aspectRatio],
  );

  const viewMode = params.viewMode === "actual" ? "actual" : "fit";

  const fileOverrides = params.fileOverrides ?? {};

  const currentIndex = activePath
    ? paths.findIndex((path) => path === activePath)
    : -1;

  const currentOverride = activePath ? fileOverrides[activePath] : undefined;

  const isSkip = Boolean(currentOverride?.skip);

  const previewUrl = activePath ? toFileSrc(activePath) : "";

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    setPixelCrop(null);
    setImageSize(null);
    setLoadError("");
    setActionError("");
  }, [activePath, isOpen]);

  useEffect(() => {
    if (!activePath || !imageSize) {
      return;
    }

    let nextCrop = DEFAULT_CROP;
    if (currentOverride?.percentRect) {
      nextCrop = fromPercentRect(currentOverride.percentRect);
    } else if (currentOverride?.rect) {
      nextCrop = toPercentCrop(currentOverride.rect, imageSize);
    } else if (params.applyMode === "percent" && params.defaultPercentRect) {
      nextCrop = fromPercentRect(params.defaultPercentRect);
    } else if (params.applyMode === "absolute" && params.defaultRect) {
      nextCrop = toPercentCrop(params.defaultRect, imageSize);
    }

    const normalized = normalizeCropToAspect(nextCrop, aspect, imageSize);
    setCrop(normalized);
  }, [activePath, aspect, imageSize, currentOverride, params.applyMode, params.defaultPercentRect, params.defaultRect]);

  useEffect(() => {
    if (!imageSize) {
      return;
    }

    const x = ((crop.x ?? 0) / 100) * imageSize.width;
    const y = ((crop.y ?? 0) / 100) * imageSize.height;
    const width = ((crop.width ?? 0) / 100) * imageSize.width;
    const height = ((crop.height ?? 0) / 100) * imageSize.height;

    setPixelCrop({
      unit: "px",
      x,
      y,
      width,
      height,
    });
  }, [crop, imageSize]);

  const handleImageLoaded = useCallback((image: HTMLImageElement) => {
    setImageSize({ width: image.naturalWidth, height: image.naturalHeight });
    return false;
  }, []);


  const saveCurrent = useCallback(() => {
    if (!activePath) {
      return;
    }

    if (!pixelCrop || pixelCrop.width <= 0 || pixelCrop.height <= 0) {
      setActionError("请先拖拽选择裁剪区域。");
      return;
    }

    const rect = toRect(pixelCrop);
    if (!rect) {
      return;
    }

    const nextOverrides = {
      ...fileOverrides,
      [activePath]: {
        ...currentOverride,
        skip: false,
        rect,
      },
    };

    onPatchParams({ fileOverrides: nextOverrides });
    setActionError("");
  }, [activePath, currentOverride, fileOverrides, onPatchParams, pixelCrop]);

  const toggleSkip = useCallback(
    (skip: boolean) => {
      if (!activePath) {
        return;
      }

      const nextOverrides = { ...fileOverrides };
      if (skip) {
        nextOverrides[activePath] = {
          ...currentOverride,
          skip: true,
        };
      } else if (currentOverride?.rect || currentOverride?.percentRect) {
        nextOverrides[activePath] = {
          ...currentOverride,
          skip: false,
        };
      } else {
        delete nextOverrides[activePath];
      }

      onPatchParams({ fileOverrides: nextOverrides });
    },
    [activePath, currentOverride, fileOverrides, onPatchParams],
  );

  const applyToAll = useCallback(async () => {
    if (!pixelCrop || pixelCrop.width <= 0 || pixelCrop.height <= 0) {
      setActionError("请先拖拽选择裁剪区域。");
      return;
    }

    if (params.applyMode === "absolute") {
      const rect = toRect(pixelCrop);
      if (!rect) {
        return;
      }

      if (
        imageSize &&
        (rect.x + rect.width > imageSize.width ||
          rect.y + rect.height > imageSize.height)
      ) {
        setActionError("当前裁剪范围超出图片尺寸，请调整后再应用。");
        return;
      }

      setIsValidating(true);
      try {
        const checks = await Promise.all(
          paths.map(async (path) => {
            try {
              const info = await getPathImageInfo(path);
              return { path, info };
            } catch {
              return { path, info: null };
            }
          }),
        );

        const invalid = checks.filter(({ info }) => {
          if (!info || !info.width || !info.height) {
            return true;
          }

          return rect.x + rect.width > info.width || rect.y + rect.height > info.height;
        });

        if (invalid.length > 0) {
          const sample = invalid[0];
          const sizeText =
            sample.info && sample.info.width && sample.info.height
              ? `${sample.info.width}x${sample.info.height}`
              : "未知尺寸";
          setActionError(
            `有 ${invalid.length} 张图片尺寸不足，无法应用绝对裁剪。示例：${sample.path}（${sizeText}）`,
          );
          return;
        }

        const nextOverrides = { ...fileOverrides };
        for (const path of paths) {
          const existing = nextOverrides[path];
          if (existing?.skip) {
            continue;
          }
          nextOverrides[path] = {
            ...existing,
            skip: false,
            rect,
          };
        }

        onPatchParams({
          defaultRect: rect,
          applyMode: "absolute",
          fileOverrides: nextOverrides,
        });
        setActionError("");
      } finally {
        setIsValidating(false);
      }
      return;
    }

    const percentRect = toPercentRect(crop);
    const nextOverrides = { ...fileOverrides };
    for (const path of paths) {
      const existing = nextOverrides[path];
      if (existing?.skip) {
        continue;
      }
      nextOverrides[path] = {
        ...existing,
        skip: false,
        percentRect,
      };
    }

    onPatchParams({
      defaultPercentRect: percentRect,
      applyMode: "percent",
      fileOverrides: nextOverrides,
    });
    setActionError("");
  }, [
    crop,
    fileOverrides,
    imageSize,
    onPatchParams,
    params.applyMode,
    paths,
    pixelCrop,
  ]);

  const goPrev = useCallback(() => {
    if (currentIndex <= 0) {
      return;
    }
    saveCurrent();
    onSelectPath(paths[currentIndex - 1]);
  }, [currentIndex, onSelectPath, paths, saveCurrent]);

  const goNext = useCallback(() => {
    if (currentIndex < 0 || currentIndex >= paths.length - 1) {
      return;
    }
    saveCurrent();
    onSelectPath(paths[currentIndex + 1]);
  }, [currentIndex, onSelectPath, paths, saveCurrent]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
      if (event.key === "Enter") {
        saveCurrent();
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, onClose, saveCurrent]);

  if (!isOpen) {
    return null;
  }

  return (
    <div className="crop-modal-overlay" role="dialog" aria-modal="true">
      <div className="crop-modal">
        <div className="crop-modal-header">
          <div>
            <h2>手动裁剪</h2>
            <p className="hint">
              {currentIndex >= 0 ? `${currentIndex + 1}/${paths.length}` : "-"}
              {activePath ? `：${activePath}` : ""}
            </p>
          </div>
          <button type="button" className="ghost" onClick={onClose}>
            关闭
          </button>
        </div>

        <div className="crop-modal-body">
          <div className={`crop-preview ${viewMode === "actual" ? "is-actual" : ""}`}>
            {loadError ? (
              <div className="thumb-placeholder">无法预览</div>
            ) : activePath ? (
              <ReactCrop
                crop={crop}
                onChange={(_, percentCrop) => setCrop(percentCrop)}
                aspect={aspect}
                keepSelection
                disabled={isRunning}
              >
                <img
                  src={previewUrl}
                  alt={activePath}
                  onLoad={(event) =>
                    handleImageLoaded(event.currentTarget as HTMLImageElement)
                  }
                  onError={() => setLoadError("无法预览")}
                />
              </ReactCrop>
            ) : (
              <div className="thumb-placeholder">暂无图片</div>
            )}
          </div>

          <div className="crop-controls">
            <div className="crop-control-block">
              <h3>裁剪信息</h3>
              <p className="hint">
                当前尺寸（像素）：
                {pixelCrop
                  ? `${Math.round(pixelCrop.width)} x ${Math.round(pixelCrop.height)}`
                  : "-"}
              </p>
            </div>

            <div className="crop-control-block">
              <h3>操作</h3>
              {actionError ? <p className="error-inline">{actionError}</p> : null}
              {isRunning ? (
                <p className="muted">任务运行中，暂不可编辑。</p>
              ) : null}
              {isValidating ? <p className="muted">正在校验图片尺寸...</p> : null}
              <div className="toolbar">
                <button
                  type="button"
                  onClick={saveCurrent}
                  disabled={isRunning || isValidating || !activePath}
                >
                  保存当前
                </button>
                <button
                  type="button"
                  className="ghost"
                  onClick={applyToAll}
                  disabled={isRunning || isValidating}
                >
                  应用到全部
                </button>
              </div>
              <label className="field inline-checkbox">
                <input
                  type="checkbox"
                  checked={isSkip}
                  disabled={isRunning}
                  onChange={(event) => toggleSkip(event.target.checked)}
                />
                <span>跳过该图（复制原图）</span>
              </label>
            </div>

            <div className="crop-control-block">
              <h3>切换图片</h3>
              <div className="toolbar">
                <button
                  type="button"
                  className="ghost"
                  onClick={goPrev}
                  disabled={isRunning || currentIndex <= 0}
                >
                  上一张
                </button>
                <button
                  type="button"
                  className="ghost"
                  onClick={goNext}
                  disabled={
                    isRunning || currentIndex < 0 || currentIndex >= paths.length - 1
                  }
                >
                  下一张
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
