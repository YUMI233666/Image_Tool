import type { RenameConfig } from "../lib/types";

interface RenameRulePanelProps {
  config: RenameConfig;
  isRunning: boolean;
  onChange: (patch: Partial<RenameConfig>) => void;
}

function clampInt(value: number, min: number, max: number): number {
  if (Number.isNaN(value)) {
    return min;
  }

  return Math.max(min, Math.min(max, Math.floor(value)));
}

function buildPreview(config: RenameConfig): string {
  if (!config.enabled) {
    return "sample.png";
  }

  const baseName = "sample";
  const extension = "png";
  const index = String(config.startIndex ?? 1).padStart(config.indexPadding ?? 0, "0");
  const date = "20260427";
  const time = "120000";

  const replaceToken = (source: string, token: string, value: string): string =>
    source.split(token).join(value);

  let rendered = config.customName?.trim() || baseName;

  if (config.mode === "template") {
    rendered = config.template ?? "{name}_{index}";
    rendered = replaceToken(rendered, "{name}", baseName);
    rendered = replaceToken(rendered, "{index}", index);
    rendered = replaceToken(rendered, "{date}", date);
    rendered = replaceToken(rendered, "{time}", time);
    rendered = replaceToken(rendered, "{ext}", extension);
  }

  const suffix = `.${extension}`;
  if (rendered.toLowerCase().endsWith(suffix)) {
    return rendered;
  }

  return `${rendered}${suffix}`;
}

export default function RenameRulePanel({
  config,
  isRunning,
  onChange,
}: RenameRulePanelProps) {
  const preview = buildPreview(config);

  return (
    <section className="panel">
      <h2>批量重命名</h2>

      <label className="field inline-checkbox">
        <input
          type="checkbox"
          checked={config.enabled}
          onChange={(event) => onChange({ enabled: event.target.checked })}
          disabled={isRunning}
        />
        <span>启用输出重命名</span>
      </label>

      {!config.enabled ? (
        <p className="muted">关闭后将保持原始文件名输出（重名自动追加序号）。</p>
      ) : (
        <>
          <label className="field">
            <span>命名模式</span>
            <select
              value={config.mode}
              onChange={(event) =>
                onChange({
                  mode: event.target.value as RenameConfig["mode"],
                })
              }
              disabled={isRunning}
            >
              <option value="custom">自定义名称</option>
              <option value="template">模板名称</option>
            </select>
          </label>

          {config.mode === "custom" ? (
            <label className="field">
              <span>文件名</span>
              <input
                value={config.customName ?? ""}
                onChange={(event) => onChange({ customName: event.target.value })}
                placeholder="例如：avatar"
                disabled={isRunning}
              />
            </label>
          ) : (
            <>
              <label className="field">
                <span>模板</span>
                <input
                  value={config.template ?? "{name}_{index}"}
                  onChange={(event) => onChange({ template: event.target.value })}
                  placeholder="例如：{name}_{date}_{index}"
                  disabled={isRunning}
                />
              </label>
              <p className="hint">
                可用变量：{"{name}"} {"{index}"} {"{date}"} {"{time}"} {"{ext}"}
              </p>
            </>
          )}

          <div className="workflow-grid-two">
            <label className="field">
              <span>起始序号</span>
              <input
                type="number"
                min={1}
                max={999999}
                value={config.startIndex ?? 1}
                onChange={(event) =>
                  onChange({
                    startIndex: clampInt(Number(event.target.value), 1, 999999),
                  })
                }
                disabled={isRunning}
              />
            </label>

            <label className="field">
              <span>序号补零位数</span>
              <input
                type="number"
                min={0}
                max={8}
                value={config.indexPadding ?? 0}
                onChange={(event) =>
                  onChange({
                    indexPadding: clampInt(Number(event.target.value), 0, 8),
                  })
                }
                disabled={isRunning}
              />
            </label>
          </div>

          <p className="hint">预览：{preview}</p>
        </>
      )}
    </section>
  );
}
