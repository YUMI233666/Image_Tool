import type { ProcessorId } from "./types";

export interface Preset { name: string; params: Record<string, unknown> }
const KEY = "art-tool-presets";

function load(): Record<string, Preset[]> {
  try { return JSON.parse(localStorage.getItem(KEY) ?? "{}"); } catch { return {}; }
}
function save(data: Record<string, Preset[]>) {
  localStorage.setItem(KEY, JSON.stringify(data));
}

export function getPresets(id: ProcessorId): Preset[] {
  return load()[id] ?? [];
}
export function savePreset(id: ProcessorId, name: string, params: Record<string, unknown>) {
  const data = load();
  const list = data[id] ?? [];
  const idx = list.findIndex(p => p.name === name);
  if (idx >= 0) list[idx] = { name, params };
  else list.push({ name, params });
  data[id] = list;
  save(data);
}
export function deletePreset(id: ProcessorId, name: string) {
  const data = load();
  data[id] = (data[id] ?? []).filter(p => p.name !== name);
  save(data);
}
