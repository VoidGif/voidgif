import { useState } from "react";
import { useAppStore } from "../stores/appStore";
import { useT } from "../stores/settingsStore";
import { resizeSession } from "../lib/ipc";
import type { SessionInfo } from "../types";

interface Props {
  onClose: () => void;
  onApplied: (s: SessionInfo) => void;
}

export default function ResizeDialog({ onClose, onApplied }: Props) {
  const t = useT();
  const session = useAppStore((s) => s.session);
  const [w, setW] = useState(String(session?.width ?? 0));
  const [h, setH] = useState(String(session?.height ?? 0));
  const [lock, setLock] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!session) return null;
  const aspect = session.width / session.height;

  const setWidth = (v: string) => {
    setW(v);
    if (lock) {
      const n = parseInt(v, 10);
      if (Number.isFinite(n) && n > 0) setH(String(Math.round(n / aspect)));
    }
  };
  const setHeight = (v: string) => {
    setH(v);
    if (lock) {
      const n = parseInt(v, 10);
      if (Number.isFinite(n) && n > 0) setW(String(Math.round(n * aspect)));
    }
  };

  const apply = async () => {
    const width = parseInt(w, 10);
    const height = parseInt(h, 10);
    if (!Number.isFinite(width) || !Number.isFinite(height) || width < 1 || height < 1) {
      setError(t("errInvalidSize"));
      return;
    }
    setBusy(true);
    try {
      onApplied(await resizeSession(width, height));
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const field =
    "mt-1 w-full rounded-lg border border-line-strong bg-void-800 px-3 py-2 text-sm text-ink-1 outline-none vg-transition focus:border-accent-500";

  return (
    <div className="vg-scrim fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
      <div className="vg-modal w-[320px] rounded-2xl border border-line-strong bg-void-900 p-6 shadow-2xl shadow-black/40">
        <h2 className="mb-5 text-lg font-semibold text-ink-1">{t("resize")}</h2>
        <div className="mb-3 flex gap-3">
          <label className="flex-1 text-sm text-ink-2">
            {t("width")}
            <input value={w} onChange={(e) => setWidth(e.target.value.replace(/\D/g, ""))} className={field} />
          </label>
          <label className="flex-1 text-sm text-ink-2">
            {t("height")}
            <input value={h} onChange={(e) => setHeight(e.target.value.replace(/\D/g, ""))} className={field} />
          </label>
        </div>
        <label className="mb-4 flex items-center gap-2 text-sm text-ink-2">
          <input
            type="checkbox"
            checked={lock}
            onChange={(e) => setLock(e.target.checked)}
            className="size-4 accent-violet-600"
          />
          {t("lockAspect")}
        </label>
        {error && <p className="mb-3 text-xs text-rose-400">{error}</p>}
        <div className="mt-6 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded-lg px-4 py-2 text-sm text-ink-2 vg-transition hover:bg-void-700 hover:text-ink-1"
          >
            {t("cancel")}
          </button>
          <button
            onClick={() => void apply()}
            disabled={busy}
            className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white shadow-sm shadow-accent-600/30 vg-transition hover:bg-accent-500 active:scale-[0.98] disabled:opacity-40"
          >
            {t("apply")}
          </button>
        </div>
      </div>
    </div>
  );
}
