import type { BatchJobReport } from "../lib/types";

interface ResultSummaryPanelProps {
  report: BatchJobReport | null;
  onOpenOutputDir: () => void;
  onOpenReport: () => void;
}

export default function ResultSummaryPanel({
  report,
  onOpenOutputDir,
  onOpenReport,
}: ResultSummaryPanelProps) {
  return (
    <section className="panel">
      <div className="panel-header">
        <h2>结果汇总</h2>
        <div className="toolbar">
          <button type="button" onClick={onOpenOutputDir} disabled={!report}>
            打开输出目录
          </button>
          <button
            type="button"
            className="ghost"
            onClick={onOpenReport}
            disabled={!report?.reportPath}
          >
            打开报告
          </button>
        </div>
      </div>

      {!report ? (
        <p className="muted">执行完成后会在这里展示统计信息与失败明细。</p>
      ) : (
        <>
          <div className="stats-grid">
            <div>
              <span>总数</span>
              <strong>{report.total}</strong>
            </div>
            <div>
              <span>成功</span>
              <strong>{report.succeeded}</strong>
            </div>
            <div>
              <span>失败</span>
              <strong>{report.failed}</strong>
            </div>
            <div>
              <span>跳过</span>
              <strong>{report.skipped}</strong>
            </div>
          </div>

          {report.failed > 0 ? (
            <div className="input-list">
              <p className="muted">失败样本（最多展示5条）</p>
              <ul>
                {report.items
                  .filter((item) => item.status === "failed")
                  .slice(0, 5)
                  .map((item) => {
                    const failedStep = item.steps?.find(
                      (step) => step.status === "failed",
                    );

                    return (
                      <li key={`${item.inputPath}-${item.durationMs}`}>
                        {item.inputPath} -
                        {failedStep
                          ? ` [${failedStep.processorId}] ${failedStep.message}`
                          : ` ${item.message}`}
                      </li>
                    );
                  })}
              </ul>
            </div>
          ) : (
            <p className="hint">没有失败项。</p>
          )}
        </>
      )}
    </section>
  );
}
