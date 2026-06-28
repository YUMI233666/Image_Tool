import type { ProcessorDescriptor, ProcessorId } from "../lib/types";

const ICONS: Record<string, string> = {
  "trim-transparent": "✂️",
  "format-convert":   "🔄",
  "compress":         "🗜️",
  "repair":           "🔧",
  "resolution-transform": "📐",
  "rename":           "✏️",
  "upscale-anime":    "⭐",
  "manual-crop":      "✂",
};

interface Props {
  processors: ProcessorDescriptor[];
  selectedProcessorId: ProcessorId;
  onSelect: (id: ProcessorId) => void;
}

export default function FunctionSelector({ processors, selectedProcessorId, onSelect }: Props) {
  return (
    <div>
      <p className="muted" style={{marginBottom:10}}>
        选择要执行的处理功能。
      </p>
      <div className="fn-grid">
        {processors.map(p => (
          <button
            key={p.id}
            type="button"
            className={"fn-card" + (selectedProcessorId === p.id ? " selected" : "")}
            onClick={() => onSelect(p.id as ProcessorId)}
            disabled={!p.enabled}
          >
            <span className="fn-card-icon">{ICONS[p.id] ?? "🖼"}</span>
            <span className="fn-card-name">{p.displayName}</span>
            <span className="fn-card-note">{p.notes}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
