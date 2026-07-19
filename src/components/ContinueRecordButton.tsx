import { useEffect, useRef, useState } from "react";
import { openRecorderContinue } from "../lib/ipc";
import { useT } from "../stores/settingsStore";
import ToolbarButton from "./ToolbarButton";
import { IconRecPlus } from "./icons";

interface Props {
  /** Whether the project has any frames ("after current" needs one). */
  hasFrames: boolean;
  /** Id of the frame the "after current frame" option inserts behind. */
  currentFrameId: number | undefined;
  onError: (msg: string) => void;
}

/**
 * Editor toolbar control that opens a small popover to choose WHERE a
 * continue-recording is spliced in (start / after current / end), then opens
 * the size-locked recorder. Not a modal — dismisses on outside click / Esc.
 */
export default function ContinueRecordButton({ hasFrames, currentFrameId, onError }: Props) {
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

  const choose = (insert: "start" | "after" | "end") => {
    setOpen(false);
    const afterId = insert === "after" ? currentFrameId : undefined;
    openRecorderContinue(insert, afterId).catch((e) => {
      const msg = String(e);
      onError(/larger than the screen/i.test(msg) ? t("errRecordingTooLarge") : msg);
    });
  };

  const option = "flex w-full items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-sm vg-transition";

  return (
    <div ref={wrapRef} className="relative flex">
      <ToolbarButton
        label={t("continueRec")}
        desc={t("tipContinueRecDesc")}
        active={open}
        onClick={() => setOpen((o) => !o)}
      >
        <IconRecPlus size={17} />
      </ToolbarButton>
      {open && (
        <div
          role="menu"
          className="vg-modal absolute left-0 top-full z-50 mt-1.5 w-52 rounded-xl border border-line-strong bg-void-850 p-1 shadow-xl shadow-black/40"
        >
          <button
            type="button"
            role="menuitem"
            onClick={() => choose("start")}
            className={`${option} text-ink-1 hover:bg-void-700`}
          >
            <span className="inline-block size-2 shrink-0 rounded-sm bg-ink-3" />
            {t("continueAtStart")}
          </button>
          <button
            type="button"
            role="menuitem"
            disabled={!hasFrames}
            onClick={() => choose("after")}
            className={`${option} text-ink-1 hover:bg-void-700 disabled:opacity-40 disabled:hover:bg-transparent`}
          >
            <span className="inline-block size-2 shrink-0 rounded-sm bg-ink-3" />
            {t("continueAfterCurrent")}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => choose("end")}
            className={`${option} bg-accent-600/15 font-medium text-accent-400 hover:bg-accent-600/25`}
          >
            <span className="inline-block size-2 shrink-0 rounded-sm bg-accent-500" />
            {t("continueAtEnd")}
          </button>
        </div>
      )}
    </div>
  );
}
