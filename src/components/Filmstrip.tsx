import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useAppStore } from "../stores/appStore";
import { frameUrl, moveGroup, reorderFrames } from "../lib/ipc";
import { useT } from "../stores/settingsStore";
import type { GroupInfo } from "../types";

const THUMB_W = 96;
const GAP = 6;

interface Props {
  current: number;
  selected: Set<number>;
  onNavigate: (index: number) => void;
  onSelect: (ids: Set<number>) => void;
  onError: (msg: string) => void;
}

/** Horizontal virtualized thumbnail strip with click-select and drag-reorder. */
export default function Filmstrip({
  current,
  selected,
  onNavigate,
  onSelect,
  onError,
}: Props) {
  const t = useT();
  const session = useAppStore((s) => s.session);
  const sessionRev = useAppStore((s) => s.sessionRev);
  const setSession = useAppStore((s) => s.setSession);

  const scrollRef = useRef<HTMLDivElement>(null);
  const anchor = useRef<number | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [dragGroup, setDragGroup] = useState<number | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);

  const frames = session?.frames ?? [];
  const groupMap = useMemo(() => {
    const m = new Map<number, GroupInfo>();
    for (const g of session?.groups ?? []) m.set(g.id, g);
    return m;
  }, [session?.groups]);

  // Frame list changed (delete/duplicate/reorder/load): the stored index no
  // longer points at the same frame — drop the shift-select anchor.
  useEffect(() => {
    anchor.current = null;
  }, [session?.frames]);

  const virtualizer = useVirtualizer({
    count: frames.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => THUMB_W + GAP,
    horizontal: true,
    overscan: 12,
  });

  // Keep the current frame centered during playback / arrow navigation so
  // the playhead position is always obvious in the strip.
  useEffect(() => {
    if (frames.length > 0) {
      virtualizer.scrollToIndex(Math.min(current, frames.length - 1), {
        align: "center",
      });
    }
  }, [current, frames.length, virtualizer]);

  const clickFrame = (index: number, e: React.MouseEvent) => {
    const id = frames[index].id;
    if (e.shiftKey && anchor.current !== null) {
      const lo = Math.min(anchor.current, index);
      const hi = Math.max(anchor.current, index);
      onSelect(new Set(frames.slice(lo, hi + 1).map((f) => f.id)));
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(selected);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      onSelect(next);
      anchor.current = index;
    } else {
      onSelect(new Set([id]));
      anchor.current = index;
    }
    onNavigate(index);
  };

  const commitReorder = async () => {
    const from = dragFrom;
    const at = dropAt;
    setDragFrom(null);
    setDropAt(null);
    // Both "before itself" and "after itself" are no-ops.
    if (from === null || at === null || at === from || at === from + 1) return;
    const order = frames.map((f) => f.id);
    const [moved] = order.splice(from, 1);
    order.splice(at > from ? at - 1 : at, 0, moved);
    try {
      setSession(await reorderFrames(order));
    } catch (e) {
      onError(String(e));
    }
  };

  // Group-chip drag: move the whole member block. `dropAt` is a full-list drop
  // index; convert to the rest-space index the backend expects by removing the
  // block members that sit before it.
  const commitGroupMove = async () => {
    const gid = dragGroup;
    const at = dropAt;
    setDragGroup(null);
    setDropAt(null);
    if (gid === null || at === null) return;
    const blockBefore = frames.filter((f, i) => f.groupId === gid && i < at).length;
    const restIndex = at - blockBefore;
    try {
      setSession(await moveGroup(gid, restIndex));
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <div className="border-t border-line bg-void-900">
      <div
        ref={scrollRef}
        className="overflow-x-auto px-3 py-3"
        style={{ height: 100 }}
      >
        <div
          className="relative h-full"
          style={{ width: virtualizer.getTotalSize() }}
        >
          {virtualizer.getVirtualItems().map((v) => {
            const frame = frames[v.index];
            const isSel = selected.has(frame.id);
            const isCur = v.index === current;
            const gid = frame.groupId ?? null;
            const group = gid != null ? groupMap.get(gid) : undefined;
            const isGroupStart =
              !!group && (v.index === 0 || frames[v.index - 1].groupId !== gid);
            const bandColor = group ? `var(--vg-group-${group.color})` : undefined;
            return (
              <div
                key={frame.id}
                draggable
                onDragStart={() => setDragFrom(v.index)}
                onDragOver={(e) => {
                  e.preventDefault();
                  // currentTarget: the thumbnail tile — e.target can be the
                  // tiny label spans, whose rects give the wrong midpoint.
                  const rect = e.currentTarget.getBoundingClientRect();
                  const before = e.clientX < rect.left + rect.width / 2;
                  setDropAt(before ? v.index : v.index + 1);
                }}
                onDragEnd={() => void commitReorder()}
                onClick={(e) => clickFrame(v.index, e)}
                className={`group absolute top-0 flex h-full cursor-pointer flex-col overflow-hidden rounded-lg border vg-transition ${
                  isCur
                    ? "border-accent-400 ring-2 ring-accent-400/80 shadow-lg shadow-accent-600/20"
                    : isSel
                      ? "border-accent-600/70 hover:border-accent-500"
                      : "border-line-strong hover:border-ink-3 hover:shadow-md hover:shadow-black/20"
                } ${isSel ? "bg-accent-600/15" : "bg-void-800"}`}
                style={{
                  left: v.start,
                  width: THUMB_W,
                }}
              >
                {/* Group color band across the top of every member tile. */}
                {group && (
                  <div
                    className="absolute inset-x-0 top-0 z-10 h-0.5"
                    style={{ background: bandColor }}
                  />
                )}
                {/* First tile carries a draggable "●N" chip that moves the
                    whole block. */}
                {isGroupStart && group && (
                  <div
                    draggable
                    onDragStart={(e) => {
                      e.stopPropagation();
                      setDragFrom(null);
                      setDragGroup(gid);
                    }}
                    onDragEnd={() => void commitGroupMove()}
                    onClick={(e) => e.stopPropagation()}
                    title={t("group") + " " + group.number}
                    className="absolute left-0.5 top-0.5 z-20 flex cursor-grab items-center gap-px rounded-full px-1 py-px text-[9px] font-bold leading-none text-white shadow-sm shadow-black/40 active:cursor-grabbing"
                    style={{ background: bandColor }}
                  >
                    <span className="text-[6px] leading-none">●</span>
                    {group.number}
                  </div>
                )}
                <img
                  src={frameUrl(frame.id, 96, sessionRev)}
                  alt=""
                  loading="lazy"
                  draggable={false}
                  className={`min-h-0 flex-1 object-contain vg-transition ${
                    isCur ? "opacity-100" : "opacity-85 group-hover:opacity-100"
                  }`}
                />
                <div className="flex items-center justify-between px-1 py-0.5 font-mono text-[11px]">
                  <span
                    className={
                      isCur
                        ? "rounded bg-accent-500 px-1 font-semibold text-white"
                        : "px-0.5 text-ink-2"
                    }
                  >
                    {v.index + 1}
                  </span>
                  <span className="text-ink-2">{frame.delayMs}ms</span>
                </div>
                {dropAt === v.index && (dragFrom !== null || dragGroup !== null) && (
                  <div className="absolute inset-y-0 left-0 w-0.5 bg-accent-400 shadow-[0_0_6px_rgba(167,139,250,0.9)]" />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
