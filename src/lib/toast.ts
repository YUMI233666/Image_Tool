import { useState, useCallback } from "react";

export type ToastType = "ok" | "error" | "warn" | "info";
export interface ToastItem { id: number; msg: string; type: ToastType }

let _next = 0;
type Setter = React.Dispatch<React.SetStateAction<ToastItem[]>>;
let _set: Setter | null = null;

export function registerToastSetter(fn: Setter) { _set = fn; }

export function toast(msg: string, type: ToastType = "info", ms = 3200) {
  if (!_set) return;
  const id = ++_next;
  _set(prev => [...prev, { id, msg, type }]);
  setTimeout(() => _set!(prev => prev.filter(t => t.id !== id)), ms);
}
export function toastOk(msg: string) { toast(msg, "ok"); }
export function toastErr(msg: string) { toast(msg, "error", 5000); }

export function useToastStore() {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const reg = useCallback((fn: Setter) => { registerToastSetter(fn); }, []);
  return { toasts, setToasts, reg };
}
