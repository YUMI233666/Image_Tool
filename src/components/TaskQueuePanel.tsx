import type { BatchProgressPayload } from "../lib/types";

interface Props{
  isRunning:boolean;progress:BatchProgressPayload|null;
  onCancel:()=>void;canCancel:boolean;
}
function pct(p:BatchProgressPayload|null){
  return(!p||p.total===0)?0:Math.round(p.processed/p.total*100);
}

export default function TaskQueuePanel({isRunning,progress,onCancel,canCancel}:Props){
  const pc=pct(progress);
  return(
    <div>
      {!isRunning&&!progress&&<p className="muted">暂无运行中的任务。点击「开始处理」启动。</p>}
      {isRunning&&!progress&&<p className="muted">任务已启动，等待后端回传进度…</p>}

      <div style={{display:"flex",alignItems:"center",justifyContent:"space-between",marginBottom:6}}>
        <span style={{fontSize:13,color:"var(--tx-m)"}}>{pc}% {progress?.currentFile?"— "+progress.currentFile.split("/").pop():""}</span>
        <button type="button" className="danger sm" onClick={onCancel} disabled={!canCancel}>⏹ 取消</button>
      </div>

      <div className="progress-track" role="progressbar" aria-valuenow={pc} aria-valuemin={0} aria-valuemax={100}>
        <div className="progress-bar" style={{width:pc+"%"}}/>
      </div>

      <div className="stats-grid">
        <div><span>进度</span><strong>{progress?.processed??0}/{progress?.total??0}</strong></div>
        <div><span>成功</span><strong style={{color:"var(--ok)"}}>{progress?.succeeded??0}</strong></div>
        <div><span>失败</span><strong style={{color:"var(--er)"}}>{progress?.failed??0}</strong></div>
        <div><span>跳过</span><strong style={{color:"var(--wn)"}}>{progress?.skipped??0}</strong></div>
      </div>

      {progress?.currentStepProcessorId&&(
        <div className="queue-item">
          <p>步骤 <span>{progress.currentStepIndex}/{progress.currentStepTotal}</span>
            &nbsp;—&nbsp;<span>{progress.currentStepProcessorId}</span></p>
          {progress.currentStepMessage&&<p className="hint">{progress.currentStepMessage}</p>}
        </div>
      )}
    </div>
  );
}
