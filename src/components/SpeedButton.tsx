import { useEffect, useRef, useState } from "react";
import { useT } from "../stores/settingsStore";
import ToolbarButton from "./ToolbarButton";
import { IconSpeed } from "./icons";

interface Props {
  /** Number of currently selected frames (0 = the op targets every frame). */
  selectedCount: number;
  disabled?: boolean;
  /** Apply a playback-speed factor (2 = twice as fast → half the delay). */
  onApply: (factor: number) => void;
}

const FACTORS = [0.25, 0.5, 1.5, 2, 4] as const;

/**
 * Toolbar control that opens a small segmented popover of playback-speed
 * presets. Applies to the current selection when there is one, otherwise to
 * every frame — the popover states which. Dismisses on outside click / Esc.
 */
export default function SpeedButton({ selectedCount, disabled, onApply }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const target =
    selectedCount > 0
      ? t("speedApplySelection", { count: selectedCount })
      : t("speedApplyAll");

  const pick = (factor: number) => {
    setOpen(false);
    onApply(factor);
  };

  return (
    <div ref={wrapRef} className="relative flex">
      <ToolbarButton
        label={t("speed")}
        desc={t("tipSpeedDesc")}
        active={open}
        disabled={disabled}
        onClick={() => setOpen((o) => !o)}
      >
        <IconSpeed size={16} />
      </ToolbarButton>
      {open && (
        <div
          role="menu"
          className="vg-modal absolute left-1/2 top-full z-50 mt-1.5 w-60 -translate-x-1/2 rounded-xl border border-line-strong bg-void-850 p-2.5 shadow-xl shadow-black/40"
        >
          <div className="mb-1 px-0.5 text-xs font-semibold text-ink-1">
            {t("speedTitle")}
          </div>
          <div className="mb-2 px-0.5 text-[11px] text-ink-3">{target}</div>
          <div className="flex gap-1">
            {FACTORS.map((f) => (
              <button
                key={f}
                type="button"
                role="menuitem"
                onClick={() => pick(f)}
                className="flex-1 rounded-md border border-line-strong bg-void-800 py-1.5 text-center font-mono text-xs text-ink-1 vg-transition hover:border-accent-500 hover:bg-accent-600/15 hover:text-accent-400"
              >
                {f}×
              </button>
            ))}
          </div>
          <p className="mt-2 px-0.5 text-[10px] leading-snug text-ink-3">
            {t("speedClampNote")}
          </p>
        </div>
      )}
    </div>
  );
}
