import type { ProcessorDescriptor, ProcessorId } from "../lib/types";

interface FunctionSelectorProps {
  processors: ProcessorDescriptor[];
  selectedProcessorId: ProcessorId;
  onSelect: (id: ProcessorId) => void;
}

export default function FunctionSelector({
  processors,
  selectedProcessorId,
  onSelect,
}: FunctionSelectorProps) {
  const selected =
    processors.find((item) => item.id === selectedProcessorId) ?? processors[0];

  return (
    <section className="panel">
      <h2>功能选择</h2>
      <p className="muted">当前版本可用：透明边缘裁剪、图像格式转换、图像压缩、图像修复、变换分辨率。</p>

      <label className="field">
        <span>处理功能</span>
        <select
          value={selectedProcessorId}
          onChange={(event) => onSelect(event.target.value as ProcessorId)}
        >
          {processors.map((processor) => (
            <option
              key={processor.id}
              value={processor.id}
              disabled={!processor.enabled}
            >
              {processor.displayName}
              {!processor.enabled ? "（暂不可用）" : ""}
            </option>
          ))}
        </select>
      </label>

      {selected ? (
        <p className="hint">{selected.notes}</p>
      ) : (
        <p className="hint">尚未加载处理器列表。</p>
      )}
    </section>
  );
}
