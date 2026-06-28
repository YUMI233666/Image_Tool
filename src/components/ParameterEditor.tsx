import { useState } from "react";
import type { ManualCropParams, ProcessorId } from "../lib/types";
import { getPresets, savePreset, deletePreset } from "../lib/presets";

const CL = (v: unknown, mn: number, mx: number): number => {
  const n = typeof v === "number" && !isNaN(v) ? v : mn;
  return Math.min(mx, Math.max(mn, Math.floor(n)));
};
const RM = 1, RX = 16384, SM = 1, SX = 100;
type RFO = Record<string, { targetWidth?: unknown; targetHeight?: unknown }>;

interface Props {
  processorId: ProcessorId | null;
  params: Record<string, unknown>;
  isRunning: boolean;
  stepLabel?: string;
  overridePaths?: string[];
  cropPaths?: string[];
  isLoadingOverrides?: boolean;
  overridesError?: string;
  isLoadingCrop?: boolean;
  cropError?: string;
  selectedInputPath?: string | null;
  onPatch: (id: ProcessorId, patch: Record<string, unknown>) => void;
  onOpenCropEditor?: (path?: string) => void;
}

export default function ParameterEditor({
  processorId, params, isRunning, stepLabel,
  overridePaths = [], cropPaths = [],
  isLoadingOverrides, overridesError, isLoadingCrop, cropError, selectedInputPath,
  onPatch, onOpenCropEditor,
}: Props) {
  const [presetName, setPresetName] = useState("");
  if (!processorId) return <p className="muted">请先在工作流中选择一个步骤。</p>;

  const p = (patch: Record<string, unknown>) => onPatch(processorId, patch);
  const presets = getPresets(processorId);

  const PresetBar = () => (
    <div className="preset-bar">
      {presets.length > 0 && (
        <select defaultValue="" onChange={e => {
          const pr = presets.find(x => x.name === e.target.value);
          if (pr) onPatch(processorId, pr.params);
        }}>
          <option value="" disabled>载入预设…</option>
          {presets.map(pr => <option key={pr.name} value={pr.name}>{pr.name}</option>)}
        </select>
      )}
      <input style={{ flex: 1, minWidth: 80 }} placeholder="预设名" value={presetName}
        onChange={e => setPresetName(e.target.value)} />
      <button type="button" className="ghost sm" onClick={() => {
        if (presetName.trim()) { savePreset(processorId, presetName.trim(), params); setPresetName(""); }
      }}>保存</button>
      {presets.some(x => x.name === presetName) && (
        <button type="button" className="danger sm" onClick={() => {
          deletePreset(processorId, presetName); setPresetName("");
        }}>删除</button>
      )}
    </div>
  );

  let body: React.ReactNode;
  switch (processorId) {
    case "trim-transparent":
      body = (<>
        <label className="field"><span>透明阈值 (0-255)</span>
          <input type="number" min={0} max={255} disabled={isRunning}
            value={CL(params.alphaThreshold, 0, 255)}
            onChange={e => p({ alphaThreshold: CL(+e.target.value, 0, 255) })} />
        </label>
        <label className="field"><span>保留边距 (px)</span>
          <input type="number" min={0} max={200} disabled={isRunning}
            value={CL(params.padding, 0, 200)}
            onChange={e => p({ padding: CL(+e.target.value, 0, 200) })} />
        </label>
      </>); break;
    case "format-convert":
      body = (<label className="field"><span>目标格式</span>
        <select value={String(params.targetFormat ?? "png")} disabled={isRunning}
          onChange={e => p({ targetFormat: e.target.value })}>
          <option value="png">PNG</option>
          <option value="jpg">JPG</option>
          <option value="webp">WEBP</option>
        </select>
      </label>); break;
    case "compress":
      body = (<>
        <label className="field"><span>压缩模式</span>
          <select value={String(params.mode ?? "balanced")} disabled={isRunning}
            onChange={e => p({ mode: e.target.value })}>
            <option value="balanced">balanced（平衡）</option>
            <option value="lossy">lossy（有损）</option>
            <option value="lossless">lossless（无损）</option>
          </select>
        </label>
        <label className="field"><span>压缩质量 (1-100)</span>
          <input type="number" min={1} max={100} disabled={isRunning}
            value={CL(params.quality, 1, 100)}
            onChange={e => p({ quality: CL(+e.target.value, 1, 100) })} />
        </label>
      </>); break;
    case "repair": {
      const mode = String(params.mode ?? "auto");
      body = (<>
        <label className="field"><span>修复模式</span>
          <select value={mode} disabled={isRunning} onChange={e => p({ mode: e.target.value })}>
            <option value="auto">自动</option><option value="denoise">去噪</option>
            <option value="scratch">划痕修复</option><option value="upscale">低分辨率增强</option>
          </select>
        </label>
        <label className="field"><span>修复强度 (1-100)</span>
          <input type="number" min={SM} max={SX} disabled={isRunning}
            value={CL(params.strength,SM,SX)} onChange={e=>p({strength:CL(+e.target.value,SM,SX)})} />
        </label>
        {mode==="upscale" && <>
          <label className="field"><span>放大倍数</span>
            <select value={String(CL(params.upscaleFactor,2,4))} disabled={isRunning}
              onChange={e=>p({upscaleFactor:CL(+e.target.value,2,4)})}>
              <option value="2">2x</option><option value="3">3x</option><option value="4">4x</option>
            </select>
          </label>
          <label className="field"><span>超分锐化强度 (1-100)</span>
            <input type="number" min={SM} max={SX} disabled={isRunning}
              value={CL(params.upscaleSharpness,SM,SX)}
              onChange={e=>p({upscaleSharpness:CL(+e.target.value,SM,SX)})} />
          </label>
        </>}
      </>); break;
    }
    case "resolution-transform": {
      const cands=overridePaths.length>0?overridePaths:[];
      const gW=CL(params.targetWidth,RM,RX),gH=CL(params.targetHeight,RM,RX);
      const sharp=CL(params.upscaleSharpness,SM,SX);
      const fov=(params.fileOverrides as RFO|undefined)??{};
      const setFOE=(path:string,en:boolean)=>{
        const nxt={...fov};if(!en)delete nxt[path];else nxt[path]={targetWidth:gW,targetHeight:gH};p({fileOverrides:nxt});
      };
      const patchFO=(path:string,patch:{targetWidth?:number;targetHeight?:number})=>{
        const cur=fov[path]??{targetWidth:gW,targetHeight:gH};
        p({fileOverrides:{...fov,[path]:{targetWidth:CL(patch.targetWidth??cur.targetWidth,RM,RX),targetHeight:CL(patch.targetHeight??cur.targetHeight,RM,RX)}}});
      };
      body=(<>
        <label className="field"><span>目标宽度 (1-16384)</span>
          <input type="number" min={RM} max={RX} disabled={isRunning} value={gW}
            onChange={e=>p({targetWidth:CL(+e.target.value,RM,RX)})} />
        </label>
        <label className="field"><span>目标高度 (1-16384)</span>
          <input type="number" min={RM} max={RX} disabled={isRunning} value={gH}
            onChange={e=>p({targetHeight:CL(+e.target.value,RM,RX)})} />
        </label>
        <label className="field"><span>超分锐化强度 (1-100)</span>
          <input type="number" min={SM} max={SX} disabled={isRunning} value={sharp}
            onChange={e=>p({upscaleSharpness:CL(+e.target.value,SM,SX)})} />
        </label>
        <p className="hint">PNG 比例不一致时透明居中填充；JPG/WEBP 保持原比例适配。</p>
        <section className="input-list resolution-override-list">
          <h3>单文件目标分辨率（可选）</h3>
          {isLoadingOverrides&&<p className="muted">加载中…</p>}
          {overridesError&&<p className="error-inline">{overridesError}</p>}
          {cands.length===0?<p className="muted">先选择输入文件可为每个文件单独设置。</p>:
            <ul className="resolution-override-items">{cands.map(path=>{
              const ov=fov[path];const en=Boolean(ov);
              const tw=CL(ov?.targetWidth??gW,RM,RX),th=CL(ov?.targetHeight??gH,RM,RX);
              return(<li key={path} className="resolution-override-item">
                <label className="inline-checkbox resolution-override-toggle">
                  <input type="checkbox" checked={en} disabled={isRunning} onChange={e=>setFOE(path,e.target.checked)}/>
                  <span className="resolution-override-path">{path}</span>
                </label>
                <div className="resolution-override-controls">
                  <label className="field"><span>宽度</span>
                    <input type="number" min={RM} max={RX} disabled={isRunning||!en} value={tw}
                      onChange={e=>patchFO(path,{targetWidth:CL(+e.target.value,RM,RX)})}/>
                  </label>
                  <label className="field"><span>高度</span>
                    <input type="number" min={RM} max={RX} disabled={isRunning||!en} value={th}
                      onChange={e=>patchFO(path,{targetHeight:CL(+e.target.value,RM,RX)})}/>
                  </label>
                </div>
              </li>);
            })}</ul>}
        </section>
      </>); break;
    }
    case "upscale-anime": {
      const sc=CL(params.scale,2,4),dn=CL(params.denoiseLevel,1,3);
      body=(<>
        <label className="field"><span>倍率</span>
          <select value={String(sc===4?4:2)} disabled={isRunning} onChange={e=>p({scale:+e.target.value===4?4:2})}>
            <option value="2">2x</option><option value="4">4x</option>
          </select>
        </label>
        <label className="field"><span>降噪等级 (1-3)</span>
          <select value={String(dn)} disabled={isRunning} onChange={e=>p({denoiseLevel:CL(+e.target.value,1,3)})}>
            <option value="1">1（厚涂保留笔触）</option>
            <option value="2">2（厚涂/半厚涂）</option>
            <option value="3">3（赛璐珞/伪厚涂）</option>
          </select>
        </label>
        <p className="hint">赛璐珞/伪厚涂建议 3；厚涂可降到 1-2 保留笔触。</p>
      </>); break;
    }
    case "manual-crop": {
      const mp=params as unknown as ManualCropParams;
      const am=mp.applyMode==="absolute"?"absolute":"percent";
      const vm=mp.viewMode==="actual"?"actual":"fit";
      const vals=Object.values(mp.fileOverrides??{});
      const confCnt=vals.filter(x=>x.skip||x.rect||x.percentRect).length;
      const skipCnt=vals.filter(x=>x.skip).length;
      body=(<>
        <label className="field"><span>锁定比例（可空）</span>
          <input type="text" placeholder="16:9 / 4:3 / 1.78" value={mp.aspectRatio??""} disabled={isRunning}
            onChange={e=>onPatch(processorId,{aspectRatio:e.target.value})}/>
        </label>
        <label className="field"><span>应用到全部模式</span>
          <select value={am} disabled={isRunning} onChange={e=>onPatch(processorId,{applyMode:e.target.value})}>
            <option value="percent">按比例（适配不同尺寸）</option>
            <option value="absolute">按绝对像素</option>
          </select>
        </label>
        <label className="field"><span>视图模式</span>
          <select value={vm} disabled={isRunning} onChange={e=>onPatch(processorId,{viewMode:e.target.value})}>
            <option value="fit">适应窗口</option>
            <option value="actual">原始大小</option>
          </select>
        </label>
        <div className="toolbar">
          <button type="button" disabled={isRunning||isLoadingCrop||cropPaths.length===0}
            onClick={()=>onOpenCropEditor?.(selectedInputPath??cropPaths[0])}>打开裁剪编辑器</button>
        </div>
        {isLoadingCrop&&<p className="muted">加载裁剪图片列表…</p>}
        {cropError&&<p className="error-inline">{cropError}</p>}
        {cropPaths.length===0?<p className="muted">先选择输入文件再配置裁剪。</p>
          :<p className="hint">已配置 {confCnt} 张，跳过 {skipCnt} 张。</p>}
        <p className="hint">裁剪坐标使用原始像素尺寸。</p>
      </>); break;
    }
    case "rename":
      body=<p className="muted">重命名规则请在「批量重命名」面板中配置。</p>; break;
    default:
      body=<p className="muted">该功能暂未定义参数。</p>;
  }
  const showPresets=processorId!=="rename"&&processorId!=="manual-crop";
  return(
    <div>
      {showPresets&&<PresetBar/>}
      {stepLabel&&<p className="hint" style={{marginBottom:10}}>{stepLabel}</p>}
      {body}
    </div>
  );
}
