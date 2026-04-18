import type { BatchProgressPayload } from "../lib/types";

interface TaskQueuePanelProps {
  isRunning: boolean;
  progress: BatchProgressPayload | null;
  onCancel: () => void;
  canCancel: boolean;
}

function getPercent(progress: BatchProgressPayload | null): number {
  if (!progress || progress.total === 0) {
    return 0;
  }

  return Math.round((progress.processed / progress.total) * 100);
}

export default function TaskQueuePanel({
  isRunning,
  progress,
  onCancel,
  canCancel,
}: TaskQueuePanelProps) {
  const percent = getPercent(progress);

  return (
    <section className="panel">
      <div className="panel-header">
        <h2>任务队列</h2>
        <button type="button" className="danger" onClick={onCancel} disabled={!canCancel}>
          取消任务
        </button>
      </div>

      {!isRunning && !progress ? (
        <p className="muted">当前没有运行中的任务。</p>
      ) : null}

      {isRunning && !progress ? (
        <p className="muted">任务已启动，正在等待进度回传...</p>
      ) : null}

      <div className="progress-track" aria-label="batch-progress-track">
        <div className="progress-bar" style={{ width: `${percent}%` }} />
      </div>

      <div className="stats-grid">
        <div>
          <span>进度</span>
          <strong>
            {progress?.processed ?? 0}/{progress?.total ?? 0}
          </strong>
        </div>
        <div>
          <span>成功</span>
          <strong>{progress?.succeeded ?? 0}</strong>
        </div>
        <div>
          <span>失败</span>
          <strong>{progress?.failed ?? 0}</strong>
        </div>
        <div>
          <span>跳过</span>
          <strong>{progress?.skipped ?? 0}</strong>
        </div>
      </div>

      {progress ? (
        <div className="queue-item">
          <p>
            当前文件: <span>{progress.currentFile || "(等待中)"}</span>
          </p>
          <p>
            状态: <span>{progress.status}</span>
          </p>
          <p className="hint">{progress.message}</p>
        </div>
      ) : null}
    </section>
  );
}
