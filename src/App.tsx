import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import BatchInputPanel from "./components/BatchInputPanel";
import CropEditorModal from "./components/CropEditorModal";
import FileInspectorPanel from "./components/FileInspectorPanel";
import FunctionSelector from "./components/FunctionSelector";
import ParameterEditor from "./components/ParameterEditor";
import RenameRulePanel from "./components/RenameRulePanel";
import ResultSummaryPanel from "./components/ResultSummaryPanel";
import Sidebar, { type NavTab } from "./components/Sidebar";
import TaskQueuePanel from "./components/TaskQueuePanel";
import ToastContainer from "./components/Toast";
import WorkflowBuilder from "./components/WorkflowBuilder";
import {
  cancelBatchJob, getPathImageInfo, listProcessors, listenBatchComplete,
  listenBatchProgress, openPathInSystem, previewDiscoveredFiles, startBatchJob,
} from "./lib/api/tauri";
import { toastOk, toastErr } from "./lib/toast";
import type {
  BatchItemReport, BatchJobReport, ManualCropParams, PathImageInfo,
  ProcessorDescriptor, ProcessorId, WorkflowStepRequest,
} from "./lib/types";
import { useTaskStore } from "./store/taskStore";

const fallbackProcessors: ProcessorDescriptor[] = [
  { id:"trim-transparent",  displayName:"裁剪透明边缘",   enabled:true, notes:"裁剪PNG透明边缘。" },
  { id:"format-convert",   displayName:"图像格式转换",   enabled:true, notes:"PNG/JPG/WEBP互转。" },
  { id:"compress",         displayName:"图像压缩",       enabled:true, notes:"JPG/PNG/WEBP压缩。" },
  { id:"repair",           displayName:"图像修复",       enabled:true, notes:"去噪/划痕/超分。" },
  { id:"resolution-transform", displayName:"变换分辨率", enabled:true, notes:"自动缩放或超分。" },
  { id:"rename",           displayName:"批量重命名",     enabled:true, notes:"自定义或模板命名。" },
  { id:"upscale-anime",    displayName:"二次元超分",     enabled:true, notes:"2x/4x AI超分。" },
  { id:"manual-crop",      displayName:"手动裁剪",       enabled:true, notes:"比例锁定逐张配置。" },
];
const INSPECT_DELAY=180;

export default function App() {
  const {
    availableProcessors, runMode, selectedProcessorId, workflowSteps, activeWorkflowStepId,
    renameConfig, inputPaths, outputDir, includeSubdirectories, maxConcurrency,
    paramsByProcessor, activeJobId, isRunning, progress, report,
    setAvailableProcessors, setRunMode, setSelectedProcessorId,
    addWorkflowStep, removeWorkflowStep, moveWorkflowStep, setActiveWorkflowStepId,
    updateWorkflowStepProcessor, setWorkflowStepEnabled, patchWorkflowStepParams,
    patchRenameConfig, setInputPaths, setOutputDir, setIncludeSubdirectories,
    setMaxConcurrency, patchParams, beginRun, setActiveJobId, setProgress,
    finishRun, resetReport,
  } = useTaskStore();

  const [theme, setTheme] = useState<"dark"|"light">(
    ()=>(localStorage.getItem("art-tool-theme") as "dark"|"light")??("dark")
  );
  const [activeTab, setActiveTab] = useState<NavTab>("input");
  const [uiError, setUiError] = useState("");
  const [selectedInputPath, setSelectedInputPath] = useState<string|null>(null);
  const [selectedOutputPath, setSelectedOutputPath] = useState<string|null>(null);
  const [inspectedInfo, setInspectedInfo] = useState<PathImageInfo|null>(null);
  const [isInspecting, setIsInspecting] = useState(false);
  const [inspectError, setInspectError] = useState("");
  const [overrideCandidates, setOverrideCandidates] = useState<string[]>([]);
  const [isLoadingOC, setIsLoadingOC] = useState(false);
  const [ocError, setOcError] = useState("");
  const [cropCandidates, setCropCandidates] = useState<string[]>([]);
  const [isLoadingCC, setIsLoadingCC] = useState(false);
  const [ccError, setCcError] = useState("");
  const [isCropOpen, setIsCropOpen] = useState(false);
  const [cropIdx, setCropIdx] = useState(0);
  const inspectIdRef=useRef(0);
  const inspectTimerRef=useRef<number|null>(null);
  const inspectCache=useRef(new Map<string,PathImageInfo>());
  const progressUL=useRef<(()=>void)|null>(null);
  const completeUL=useRef<(()=>void)|null>(null);
  const bindPromise=useRef<Promise<void>|null>(null);
  const startRef=useRef<(()=>Promise<void>)|null>(null);

  // Theme persistence
  useEffect(()=>{
    document.documentElement.setAttribute('data-theme',theme);
    localStorage.setItem('art-tool-theme',theme);
  },[theme]);

  // Keyboard shortcuts
  useEffect(()=>{
    const down=(e:KeyboardEvent)=>{
      if((e.ctrlKey||e.metaKey)&&e.key==='r'){e.preventDefault();startRef.current?.();}
      if(e.key==='Escape'&&isRunning){cancelBatchJob(activeJobId!).catch(()=>{});}
    };
    window.addEventListener('keydown',down);
    return()=>window.removeEventListener('keydown',down);
  },[isRunning,activeJobId]);

  // Load processors
  useEffect(()=>{
    let m=true;
    listProcessors().then(items=>{if(m)setAvailableProcessors(items.length?items:fallbackProcessors);})
      .catch(()=>{if(m)setAvailableProcessors(fallbackProcessors);});
    return()=>{m=false;};
  },[setAvailableProcessors]);

  const handleFinishRun=useCallback((r:BatchJobReport)=>{
    finishRun(r);
    if(r.failed===0) toastOk('处理完成！成功 '+r.succeeded+' 张');
    else toastErr('完成，但有 '+r.failed+' 张失败');
    setActiveTab('results');
  },[finishRun]);

  const ensureListeners=useCallback(async()=>{
    if(progressUL.current&&completeUL.current)return;
    if(bindPromise.current){await bindPromise.current;return;}
    bindPromise.current=(async()=>{
      const ul1=await listenBatchProgress(p=>{setActiveJobId(p.jobId);setProgress(p);});
      const ul2=await listenBatchComplete(r=>{handleFinishRun(r);});
      progressUL.current=ul1;completeUL.current=ul2;
    })();
    try{await bindPromise.current;}finally{bindPromise.current=null;}
  },[handleFinishRun,setActiveJobId,setProgress]);

  useEffect(()=>{
    ensureListeners().catch(()=>{});
    return()=>{progressUL.current?.();completeUL.current?.();progressUL.current=null;completeUL.current=null;};
  },[ensureListeners]);

  // Inspect cleanup
  useEffect(()=>()=>{if(inspectTimerRef.current!==null)window.clearTimeout(inspectTimerRef.current);},[]);

  // Active processor id for params/overrides
  const activeWF=useMemo<WorkflowStepRequest|null>(()=>{
    if(!workflowSteps.length)return null;
    return workflowSteps.find(s=>s.stepId===activeWorkflowStepId)??workflowSteps[0];
  },[activeWorkflowStepId,workflowSteps]);
  const activeWFIdx=useMemo(()=>workflowSteps.findIndex(s=>s.stepId===activeWorkflowStepId),[activeWorkflowStepId,workflowSteps]);
  const paramProcId=runMode==='quick'?selectedProcessorId:(activeWF?.processorId??null);
  const selectedParams=runMode==='quick'?paramsByProcessor[selectedProcessorId]:(activeWF?.params??{});
  const allProcs=availableProcessors.length?availableProcessors:fallbackProcessors;

  // Override candidates
  useEffect(()=>{
    if(paramProcId!=='resolution-transform'||inputPaths.length===0){setOverrideCandidates([]);setIsLoadingOC(false);setOcError('');return;}
    let cancel=false;setIsLoadingOC(true);setOcError('');
    previewDiscoveredFiles('resolution-transform',inputPaths,includeSubdirectories)
      .then(p=>{if(!cancel)setOverrideCandidates(p);})
      .catch(e=>{if(!cancel){setOcError(e instanceof Error?e.message:'加载失败');setOverrideCandidates(inputPaths);}})
      .finally(()=>{if(!cancel)setIsLoadingOC(false);});
    return()=>{cancel=true;};
  },[includeSubdirectories,inputPaths,paramProcId]);

  // Crop candidates
  useEffect(()=>{
    if(paramProcId!=='manual-crop'||inputPaths.length===0){setCropCandidates([]);setIsLoadingCC(false);setCcError('');return;}
    let cancel=false;setIsLoadingCC(true);setCcError('');
    previewDiscoveredFiles('manual-crop',inputPaths,includeSubdirectories)
      .then(p=>{if(!cancel)setCropCandidates(p);})
      .catch(e=>{if(!cancel){setCcError(e instanceof Error?e.message:'加载失败');setCropCandidates(inputPaths);}})
      .finally(()=>{if(!cancel)setIsLoadingCC(false);});
    return()=>{cancel=true;};
  },[includeSubdirectories,inputPaths,paramProcId]);

  const cropPaths=cropCandidates.length>0?cropCandidates:inputPaths;
  const activeCropPath=cropPaths[cropIdx]??null;

  const inspectPath=async(path:string)=>{
    const rid=++inspectIdRef.current;setInspectError('');
    const cached=inspectCache.current.get(path);
    if(cached){setIsInspecting(false);setInspectedInfo(cached);return;}
    if(inspectTimerRef.current!==null){window.clearTimeout(inspectTimerRef.current);inspectTimerRef.current=null;}
    inspectTimerRef.current=window.setTimeout(()=>{if(inspectIdRef.current===rid)setIsInspecting(true);},INSPECT_DELAY);
    try{
      const info=await getPathImageInfo(path);
      if(inspectIdRef.current!==rid)return;
      inspectCache.current.set(path,info);setInspectedInfo(info);
    }catch(e){
      if(inspectIdRef.current!==rid)return;
      setInspectedInfo(null);setInspectError(e instanceof Error?e.message:'读取失败');
    }finally{
      if(inspectIdRef.current===rid){if(inspectTimerRef.current!==null){window.clearTimeout(inspectTimerRef.current);inspectTimerRef.current=null;}setIsInspecting(false);}
    }
  };

  const handleSelectIn=(path:string)=>{if(selectedInputPath===path&&!selectedOutputPath)return;setSelectedInputPath(path);setSelectedOutputPath(null);inspectPath(path).catch(()=>{});};
  const handleSelectOut=(path:string)=>{if(selectedOutputPath===path&&!selectedInputPath)return;setSelectedOutputPath(path);setSelectedInputPath(null);inspectPath(path).catch(()=>{});};

  useEffect(()=>{if(selectedInputPath&&!inputPaths.includes(selectedInputPath)){setSelectedInputPath(null);setInspectedInfo(null);setInspectError('');};},[inputPaths,selectedInputPath]);

  const outputItems=useMemo<BatchItemReport[]>(()=>report?report.items.filter(i=>Boolean(i.outputPath)):[],[report]);
  useEffect(()=>{if(selectedOutputPath&&!outputItems.some(i=>i.outputPath===selectedOutputPath)){setSelectedOutputPath(null);setInspectedInfo(null);setInspectError('');};},[outputItems,selectedOutputPath]);

  const patchCurrentParams=useCallback((id:ProcessorId,patch:Record<string,unknown>)=>{
    if(runMode==='quick'){patchParams(id,patch);return;}
    if(!activeWF||activeWF.processorId!==id)return;
    patchWorkflowStepParams(activeWF.stepId,patch);
  },[activeWF,patchParams,patchWorkflowStepParams,runMode]);

  const start=async()=>{
    setUiError('');resetReport();
    if(!inputPaths.length){setUiError('请先选择输入文件或目录。');return;}
    if(!outputDir.trim()){setUiError('请先选择输出目录。');return;}
    if(runMode==='quick'){
      const proc=allProcs.find(p=>p.id===selectedProcessorId);
      if(!proc?.enabled){setUiError('当前功能暂不可用。');return;}
    } else {
      if(!workflowSteps.length){setUiError('工作流至少需要一个步骤。');return;}
      const bad=workflowSteps.find(s=>s.enabled!==false&&!allProcs.find(p=>p.id===s.processorId)?.enabled);
      if(bad){setUiError('工作流包含不可用步骤: '+bad.processorId);return;}
    }
    try{await ensureListeners();}catch{setUiError('监听初始化失败，请重试。');return;}
    beginRun();setProgress({jobId:activeJobId??'pending',processed:0,total:0,succeeded:0,failed:0,skipped:0,cancelled:0,currentFile:'',status:'running',message:'任务已启动…'});
    setActiveTab('run');
    const wfPayload=runMode==='workflow'?workflowSteps.map(s=>({stepId:s.stepId,processorId:s.processorId,enabled:s.enabled??true,params:s.params})):undefined;
    try{
      const r=await startBatchJob({processorId:runMode==='quick'?selectedProcessorId:(workflowSteps[0]?.processorId??selectedProcessorId),inputPaths,outputDir,params:runMode==='quick'?selectedParams:{},workflowSteps:wfPayload,renameConfig:renameConfig.enabled?renameConfig:undefined,includeSubdirectories,maxConcurrency,writeReport:true});
      handleFinishRun(r);
    }catch(e){
      const msg=e instanceof Error?e.message:'任务启动失败';
      setUiError(msg);toastErr(msg);
      finishRun({jobId:activeJobId??'',processorId:runMode==='workflow'?'workflow':selectedProcessorId,startedAt:new Date().toISOString(),finishedAt:new Date().toISOString(),total:0,succeeded:0,failed:0,skipped:0,cancelled:0,items:[]});
    }
  };

  // store latest start in ref for keyboard shortcut
  useEffect(()=>{startRef.current=start;});

  const cancel=async()=>{
    if(!activeJobId)return;
    try{await cancelBatchJob(activeJobId);}catch{setUiError('取消失败，请稍后重试。');}
  };

  const openOutputDir=async()=>{
    if(!outputDir)return;
    try{await openPathInSystem(outputDir);setUiError('');}
    catch(e){setUiError(e instanceof Error?e.message:'打开输出目录失败。');}
  };
  const openReport=async()=>{
    if(!report?.reportPath)return;
    try{await openPathInSystem(report.reportPath);setUiError('');}
    catch(e){setUiError(e instanceof Error?e.message:'打开报告失败。');}
  };

  const openCropEditor=(path?:string)=>{
    const tgt=path??cropPaths[0];if(!tgt)return;
    const idx=cropPaths.indexOf(tgt);setCropIdx(idx>=0?idx:0);handleSelectIn(tgt);setIsCropOpen(true);
  };
  const handleCropSelect=(path:string)=>{const i=cropPaths.indexOf(path);if(i>=0)setCropIdx(i);handleSelectIn(path);};
  useEffect(()=>{if(cropIdx>=cropPaths.length)setCropIdx(0);},[cropIdx,cropPaths.length]);

  return (
    <div className="app-root">
      <Sidebar
        active={activeTab} onNav={setActiveTab}
        isRunning={isRunning} failedCount={report?.failed??0}
        onStart={start} onCancel={cancel}
        theme={theme} onThemeToggle={()=>setTheme(t=>t==="dark"?"light":"dark")}
      />
      <div className="main-content">
        <div className="main-scroll">
          {uiError&&<div className="error-banner">{uiError}</div>}

          {activeTab==="input"&&<>
            <section className="panel">
              <div className="panel-h"><h2>输入与输出</h2></div>
              <BatchInputPanel inputPaths={inputPaths} outputDir={outputDir}
                includeSubdirectories={includeSubdirectories} maxConcurrency={maxConcurrency}
                isRunning={isRunning} onInputPathsChange={setInputPaths} onOutputDirChange={setOutputDir}
                onIncludeSubdirectoriesChange={setIncludeSubdirectories} onMaxConcurrencyChange={setMaxConcurrency}/>
            </section>
            <section className="panel">
              <div className="panel-h"><h2>文件检查</h2></div>
              <FileInspectorPanel inputPaths={inputPaths} outputItems={outputItems}
                selectedInputPath={selectedInputPath} selectedOutputPath={selectedOutputPath}
                inspectedInfo={inspectedInfo} isInspecting={isInspecting} inspectError={inspectError}
                onSelectInputPath={handleSelectIn} onSelectOutputPath={handleSelectOut}/>
            </section>
          </>}

          {activeTab==="function"&&<>
            <section className="panel">
              <div className="panel-h"><h2>运行模式</h2></div>
              <label className="field">
                <span>处理模式</span>
                <select value={runMode} disabled={isRunning}
                  onChange={e=>setRunMode(e.target.value as "quick"|"workflow")}>
                  <option value="quick">快捷模式（单功能）</option>
                  <option value="workflow">工作流模式（多步骤）</option>
                </select>
              </label>
              <p className="hint">快捷模式适合单一任务，工作流模式可串联多个步骤。</p>
            </section>
            {runMode==="quick"?(
              <section className="panel">
                <div className="panel-h"><h2>功能选择</h2></div>
                <FunctionSelector processors={allProcs} selectedProcessorId={selectedProcessorId}
                  onSelect={id=>setSelectedProcessorId(id as ProcessorId)}/>
              </section>
            ):(
              <WorkflowBuilder processors={allProcs} steps={workflowSteps}
                activeStepId={activeWF?.stepId??null} isRunning={isRunning}
                onSelectStep={setActiveWorkflowStepId} onAddStep={addWorkflowStep}
                onRemoveStep={removeWorkflowStep} onMoveStep={moveWorkflowStep}
                onChangeStepProcessor={updateWorkflowStepProcessor} onToggleStepEnabled={setWorkflowStepEnabled}/>
            )}
          </>}

          {activeTab==="params"&&<>
            <section className="panel">
              <div className="panel-h">
                <h2>参数设置{runMode==="workflow"&&activeWFIdx>=0?"（步骤 "+(activeWFIdx+1)+"）":""}</h2>
              </div>
              <ParameterEditor processorId={paramProcId} params={selectedParams} isRunning={isRunning}
                overridePaths={overrideCandidates} cropPaths={cropPaths}
                isLoadingOverrides={isLoadingOC} overridesError={ocError}
                isLoadingCrop={isLoadingCC} cropError={ccError} selectedInputPath={selectedInputPath}
                onPatch={patchCurrentParams} onOpenCropEditor={openCropEditor}/>
            </section>
            <section className="panel">
              <div className="panel-h"><h2>批量重命名</h2></div>
              <RenameRulePanel config={renameConfig} isRunning={isRunning} onChange={patchRenameConfig}/>
            </section>
          </>}

          {activeTab==="run"&&(
            <section className="panel">
              <div className="panel-h"><h2>任务进度</h2></div>
              <TaskQueuePanel isRunning={isRunning} progress={progress} onCancel={cancel}
                canCancel={Boolean(isRunning&&activeJobId)}/>
            </section>
          )}

          {activeTab==="results"&&(
            <section className="panel">
              <div className="panel-h"><h2>结果汇总</h2></div>
              <ResultSummaryPanel report={report} onOpenOutputDir={openOutputDir} onOpenReport={openReport}/>
            </section>
          )}
        </div>
      </div>

      <ToastContainer/>
      <CropEditorModal isOpen={isCropOpen} isRunning={isRunning} paths={cropPaths}
        activePath={activeCropPath} params={selectedParams as unknown as ManualCropParams}
        onPatchParams={patch=>patchCurrentParams("manual-crop",patch)}
        onSelectPath={handleCropSelect} onClose={()=>setIsCropOpen(false)}/>
    </div>
  );
}
