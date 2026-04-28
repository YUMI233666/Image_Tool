import { useMemo } from "react";
import type {
  ProcessorDescriptor,
  ProcessorId,
  WorkflowStepRequest,
} from "../lib/types";

interface WorkflowBuilderProps {
  processors: ProcessorDescriptor[];
  steps: WorkflowStepRequest[];
  activeStepId: string | null;
  isRunning: boolean;
  onSelectStep: (stepId: string) => void;
  onAddStep: (processorId: ProcessorId) => void;
  onRemoveStep: (stepId: string) => void;
  onMoveStep: (stepId: string, direction: "up" | "down") => void;
  onChangeStepProcessor: (stepId: string, processorId: ProcessorId) => void;
}

const fallbackProcessorId: ProcessorId = "trim-transparent";

export default function WorkflowBuilder({
  processors,
  steps,
  activeStepId,
  isRunning,
  onSelectStep,
  onAddStep,
  onRemoveStep,
  onMoveStep,
  onChangeStepProcessor,
}: WorkflowBuilderProps) {
  const enabledProcessors = useMemo(
    () => processors.filter((item) => item.enabled),
    [processors],
  );

  const defaultProcessorId =
    steps[steps.length - 1]?.processorId ??
    enabledProcessors[0]?.id ??
    fallbackProcessorId;

  const handleAddStep = () => {
    onAddStep(defaultProcessorId);
  };

  return (
    <section className="panel">
      <div className="panel-header">
        <h2>工作流编排</h2>
        <button
          type="button"
          className="workflow-add-button"
          onClick={handleAddStep}
          disabled={isRunning || enabledProcessors.length === 0}
        >
          添加步骤
        </button>
      </div>

      <p className="hint">新增步骤后可在列表中选择处理器类型与调整顺序。</p>

      {steps.length === 0 ? (
        <p className="muted">当前没有步骤，请先添加一个处理步骤。</p>
      ) : (
        <ul className="workflow-step-list">
          {steps.map((step, index) => {
            const selected = activeStepId === step.stepId;
            const currentProcessor =
              processors.find((item) => item.id === step.processorId) ??
              processors[0];

            return (
              <li
                key={step.stepId}
                className={`workflow-step-item${selected ? " is-selected" : ""}`}
              >
                <button
                  type="button"
                  className="workflow-step-main"
                  onClick={() => onSelectStep(step.stepId)}
                  disabled={isRunning}
                >
                  <span className="workflow-step-index">步骤 {index + 1}</span>
                  <span>{currentProcessor?.displayName ?? step.processorId}</span>
                </button>

                <div className="workflow-step-actions">
                  <select
                    value={step.processorId}
                    onChange={(event) =>
                      onChangeStepProcessor(step.stepId, event.target.value as ProcessorId)
                    }
                    disabled={isRunning}
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
                  <button
                    type="button"
                    className="ghost"
                    onClick={() => onMoveStep(step.stepId, "up")}
                    disabled={isRunning || index === 0}
                  >
                    上移
                  </button>
                  <button
                    type="button"
                    className="ghost"
                    onClick={() => onMoveStep(step.stepId, "down")}
                    disabled={isRunning || index === steps.length - 1}
                  >
                    下移
                  </button>
                  <button
                    type="button"
                    className="danger"
                    onClick={() => onRemoveStep(step.stepId)}
                    disabled={isRunning || steps.length === 1}
                  >
                    删除
                  </button>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
