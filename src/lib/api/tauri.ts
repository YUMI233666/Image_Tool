import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import type {
  BatchJobReport,
  BatchProgressPayload,
  CropCompletePayload,
  CropErrorPayload,
  CropImageRequest,
  CropImageResponse,
  CropStartPayload,
  PathImageInfo,
  ProcessorDescriptor,
  ProcessorId,
  StartBatchJobRequest,
  UpscaleAnimeRequest,
  UpscaleAnimeResponse,
  UpscaleCompletePayload,
  UpscaleErrorPayload,
  UpscaleProgressPayload,
  UpscaleStartPayload,
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

export async function getPathImageInfo(path: string): Promise<PathImageInfo> {
  return invoke<PathImageInfo>("get_path_image_info", { path });
}

export async function previewDiscoveredFiles(
  processorId: ProcessorId,
  inputPaths: string[],
  includeSubdirectories: boolean,
): Promise<string[]> {
  return invoke<string[]>("preview_discovered_files", {
    request: {
      processorId,
      inputPaths,
      includeSubdirectories,
    },
  });
}

export async function upscaleAnime(
  request: UpscaleAnimeRequest,
): Promise<UpscaleAnimeResponse> {
  return invoke<UpscaleAnimeResponse>("upscale_anime", { request });
}

export async function cropImage(
  request: CropImageRequest,
): Promise<CropImageResponse> {
  return invoke<CropImageResponse>("crop_image", { request });
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

export async function listenUpscaleStart(
  callback: (payload: UpscaleStartPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<UpscaleStartPayload>("upscale-start", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}

export async function listenUpscaleProgress(
  callback: (payload: UpscaleProgressPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<UpscaleProgressPayload>(
    "upscale-progress",
    (event) => {
      if (event.payload) {
        callback(event.payload);
      }
    },
  );

  return unlisten;
}

export async function listenUpscaleComplete(
  callback: (payload: UpscaleCompletePayload) => void,
): Promise<() => void> {
  const unlisten = await listen<UpscaleCompletePayload>(
    "upscale-complete",
    (event) => {
      if (event.payload) {
        callback(event.payload);
      }
    },
  );

  return unlisten;
}

export async function listenUpscaleError(
  callback: (payload: UpscaleErrorPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<UpscaleErrorPayload>("upscale-error", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}

export async function listenCropStart(
  callback: (payload: CropStartPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<CropStartPayload>("crop-start", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}

export async function listenCropComplete(
  callback: (payload: CropCompletePayload) => void,
): Promise<() => void> {
  const unlisten = await listen<CropCompletePayload>("crop-complete", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}

export async function listenCropError(
  callback: (payload: CropErrorPayload) => void,
): Promise<() => void> {
  const unlisten = await listen<CropErrorPayload>("crop-error", (event) => {
    if (event.payload) {
      callback(event.payload);
    }
  });

  return unlisten;
}
