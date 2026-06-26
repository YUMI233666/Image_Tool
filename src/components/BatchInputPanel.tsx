import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import { appWindow } from "@tauri-apps/api/window";

const IMG_EXTS=new Set(["png","jpg","jpeg","webp","bmp","tiff","gif"]);
const isImg=(p:string)=>IMG_EXTS.has(p.split(".").pop()?.toLowerCase()??"x");
function norm(v:string|string[]|null,def:string[]=[]):string[]{
  if(v===null)return def; return Array.isArray(v)?v:[v];
}

interface Props{
  inputPaths:string[];outputDir:string;includeSubdirectories:boolean;
  maxConcurrency:number;isRunning:boolean;
  onInputPathsChange:(p:string[])=>void;onOutputDirChange:(p:string)=>void;
  onIncludeSubdirectoriesChange:(v:boolean)=>void;onMaxConcurrencyChange:(v:number)=>void;
}

export default function BatchInputPanel({
  inputPaths,outputDir,includeSubdirectories,maxConcurrency,isRunning,
  onInputPathsChange,onOutputDirChange,onIncludeSubdirectoriesChange,onMaxConcurrencyChange,
}:Props){
  const [dragover,setDragover]=useState(false);
  const unlistenRef=useRef<(()=>void)|null>(null);

  useEffect(()=>{
    let mounted=true;
    appWindow.onFileDropEvent(e=>{
      if(!mounted||isRunning)return;
      if(e.payload.type==="hover"){setDragover(true);}
      else if(e.payload.type==="drop"){
        setDragover(false);
        const paths=(e.payload.paths??[]).filter(isImg);
        if(paths.length>0)onInputPathsChange([...new Set([...inputPaths,...paths])]);
      } else {setDragover(false);}
    }).then(fn=>{if(mounted)unlistenRef.current=fn;}).catch(()=>{});
    return()=>{mounted=false;unlistenRef.current?.();};
  },[isRunning,inputPaths,onInputPathsChange]);

  const pickFiles=async()=>{
    const r=await open({multiple:true,filters:[{name:"Images",extensions:["png","jpg","jpeg","webp","bmp"]}]});
    const s=norm(r,inputPaths);onInputPathsChange(s);
  };
  const pickFolder=async()=>{
    const r=await open({directory:true,multiple:false});
    const s=norm(r);if(s.length>0)onInputPathsChange(s);
  };
  const pickOut=async()=>{
    const r=await open({directory:true,multiple:false});
    const s=norm(r);if(s.length>0)onOutputDirChange(s[0]);
  };

  return(
    <div>
      <div className={"drop-zone"+(dragover?" dragover":"")} onClick={isRunning?undefined:pickFiles}>
        <div className="drop-zone-icon">📂</div>
        <div className="drop-zone-text">{dragover?"松开即可导入":"拖拽图片 / 点击选择文件"}</div>
        <div className="drop-zone-hint">支持 PNG / JPG / WEBP / BMP 格式，可拖入文件或文件夹</div>
      </div>

      <div className="toolbar">
        <button type="button" onClick={pickFiles} disabled={isRunning}>选择图片文件</button>
        <button type="button" onClick={pickFolder} disabled={isRunning}>选择文件夹</button>
        <button type="button" className="ghost" disabled={isRunning||inputPaths.length===0}
          onClick={()=>onInputPathsChange([])}>清空</button>
      </div>

      {inputPaths.length>0&&(
        <div className="input-list">
          <p className="muted">已选 {inputPaths.length} 项</p>
          <ul>{inputPaths.slice(0,5).map(p=><li key={p}>{p}</li>)}</ul>
          {inputPaths.length>5&&<p className="hint">仅展示前5项，实际处理全部。</p>}
        </div>
      )}

      <label className="field">
        <span>输出目录</span>
        <div className="inline-actions">
          <input value={outputDir} disabled={isRunning} placeholder="请选择输出目录"
            onChange={e=>onOutputDirChange(e.target.value)}/>
          <button type="button" onClick={pickOut} disabled={isRunning}>浏览</button>
        </div>
      </label>

      <label className="field inline-checkbox">
        <input type="checkbox" checked={includeSubdirectories} disabled={isRunning}
          onChange={e=>onIncludeSubdirectoriesChange(e.target.checked)}/>
        <span>递归处理子目录</span>
      </label>

      <label className="field">
        <span>最大并发数</span>
        <input type="number" min={1} max={64} value={maxConcurrency} disabled={isRunning}
          onChange={e=>onMaxConcurrencyChange(+e.target.value)}/>
      </label>
    </div>
  );
}
