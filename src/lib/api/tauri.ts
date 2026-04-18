import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import type {
  BatchJobReport,
  BatchProgressPayload,
  ProcessorDescriptor,
  StartBatchJobRequest,
} from "../types";

export async function listProcessors(): Promise<ProcessorDescriptor[]> {
  return invoke<ProcessorDescriptor[]>("list_processors");
}

export async function startBatchJob(
  request: StartBatchJobRequest,
): Promise<BatchJobReport> {
  return invoke<BatchJobReport>("start_batch_job", { request });
}

export async function cancelBatchJob(jobId: string): Promise<void> {
  await invoke("cancel_batch_job", { jobId });
}

export async function openPathInSystem(path: string): Promise<void> {
  await invoke("open_path_in_system", { path });
}

export async function listenBatchProgress(
  callback: (payload: BatchProgressPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<BatchProgressPayload>(
    "batch-progress",
    (event) => {
      if (event.payload) {
        callback(event.payload);
      }
    },
  );

  return unlisten;
}

export async function listenBatchComplete(
  callback: (payload: BatchJobReport) => void,
): Promise<() => void> {
  const unlisten = await listen<BatchJobReport>("batch-complete", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}
