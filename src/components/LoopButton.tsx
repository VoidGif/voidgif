import { useEffect, useRef, useState } from "react";
import { useT } from "../stores/settingsStore";
import ToolbarButton from "./ToolbarButton";
import { IconLoop } from "./icons";

interface Props {
  frameCount: number;
  /** True while the loop-seam preview playback mode is active. */
  seamActive: boolean;
  /** Append the reverse interior (needs >= 3 frames). */
  onPingpong: () => void;
  /** Set the last frame's delay to 1000 ms (needs >= 1 frame). */
  onEndFreeze: () => void;
  /** Toggle the frontend seam-preview playback (needs >= 2 frames). */
  onToggleSeam: () => void;
}

/**
 * Toolbar popover with the three loop-finishing tools. Ping-pong and the end
 * freeze are one-shot actions; the seam preview is a toggle whose active state
 * is reflected both here and on the preview counter chip. Dismisses on outside
 * click / Esc (leaving the seam-preview toggle state untouched).
 */
export default function LoopButton({
  frameCount,
  seamActive,
  onPingpong,
  onEndFreeze,
  onToggleSeam,
}: Props) {
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

  const option =
    "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm vg-transition disabled:opacity-40 disabled:hover:bg-transparent";

  const act = (fn: () => void) => {
    setOpen(false);
    fn();
  };

  return (
    <div ref={wrapRef} className="relative flex">
      <ToolbarButton
        label={t("loopTools")}
        desc={t("tipLoopDesc")}
        active={open || seamActive}
        onClick={() => setOpen((o) => !o)}
      >
        <IconLoop size={16} />
      </ToolbarButton>
      {open && (
        <div
          role="menu"
          className="vg-modal absolute left-1/2 top-full z-50 mt-1.5 w-56 -translate-x-1/2 rounded-xl border border-line-strong bg-void-850 p-1 shadow-xl shadow-black/40"
        >
          <button
            type="button"
            role="menuitem"
            disabled={frameCount < 3}
            onClick={() => act(onPingpong)}
            className={`${option} text-ink-1 hover:bg-void-700`}
          >
            <IconLoop size={15} className="shrink-0 text-ink-3" />
            {t("loopPingpong")}
          </button>
          <button
            type="button"
            role="menuitem"
            disabled={frameCount < 1}
            onClick={() => act(onEndFreeze)}
            className={`${option} text-ink-1 hover:bg-void-700`}
          >
            <span className="grid size-[15px] shrink-0 place-items-center text-[11px] leading-none text-ink-3">
              1s
            </span>
            {t("loopEndFreeze")}
          </button>
          <button
            type="button"
            role="menuitem"
            aria-pressed={seamActive}
            disabled={frameCount < 2}
            onClick={() => act(onToggleSeam)}
            className={`${option} ${
              seamActive
                ? "bg-accent-600/15 font-medium text-accent-400 hover:bg-accent-600/25"
                : "text-ink-1 hover:bg-void-700"
            }`}
          >
            <span
              className={`inline-block size-2 shrink-0 rounded-full ${
                seamActive ? "bg-accent-500" : "bg-ink-3"
              }`}
            />
            {t("loopSeamPreview")}
          </button>
        </div>
      )}
    </div>
  );
}
