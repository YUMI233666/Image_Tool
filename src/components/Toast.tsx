import { useEffect } from "react";
import { useToastStore, type ToastItem } from "../lib/toast";

const ICONS: Record<string, string> = { ok:"✓", error:"✕", warn:"⚠", info:"ℹ" };

function ToastEntry({ t }: { t: ToastItem }) {
  return (
    <div className={"toast " + t.type} role="alert">
      <span>{ICONS[t.type]}</span>
      <span>{t.msg}</span>
    </div>
  );
}

export default function ToastContainer() {
  const { toasts, setToasts, reg } = useToastStore();
  useEffect(() => { reg(setToasts); }, [reg, setToasts]);
  if (toasts.length === 0) return null;
  return (
    <div className="toast-container">
      {toasts.map(t => <ToastEntry key={t.id} t={t} />)}
    </div>
  );
}
