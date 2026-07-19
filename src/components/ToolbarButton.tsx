import type { ReactNode } from "react";

interface Props {
  /** Action name — first tooltip line and the aria-label. */
  label: string;
  /** One-line explanation — second tooltip line. */
  desc?: string;
  /** Keyboard shortcut chip rendered next to the name (e.g. "Space"). */
  hotkey?: string;
  /** Tooltip alignment relative to the button (avoid window-edge clipping). */
  align?: "start" | "center" | "end";
  /** Toggled/active visual state (e.g. crop mode). */
  active?: boolean;
  /** Tiny superscript count badge on the icon (hidden when 0/undefined). */
  badge?: number;
  disabled?: boolean;
  onClick?: () => void;
  children: ReactNode;
}

/**
 * Compact icon-only toolbar button (34px hit area) with a rich CSS-only
 * hover/focus tooltip: name + hotkey chip + one-line description. Replaces
 * native `title` in the editor toolbar so labels can never truncate.
 */
export default function ToolbarButton({
  label,
  desc,
  hotkey,
  align = "center",
  active = false,
  badge,
  disabled,
  onClick,
  children,
}: Props) {
  const tipAlign =
    align === "start" ? "vg-tip-start" : align === "end" ? "vg-tip-end" : "";
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      className={`vg-tt relative flex size-8.5 shrink-0 items-center justify-center rounded-lg vg-transition active:scale-95 disabled:opacity-30 ${
        active
          ? "bg-accent-600/25 text-accent-400"
          : "text-ink-2 hover:bg-void-700 hover:text-ink-1 disabled:hover:bg-transparent disabled:hover:text-ink-2"
      }`}
    >
      {children}
      {badge != null && badge > 0 && (
        <span className="absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-accent-600 px-1 text-[10px] font-semibold leading-none text-white">
          {badge}
        </span>
      )}
      <span className={`vg-tip ${tipAlign}`} role="tooltip">
        <span className="vg-tip-name">
          {label}
          {hotkey && <kbd>{hotkey}</kbd>}
        </span>
        {desc && <span className="vg-tip-desc">{desc}</span>}
      </span>
    </button>
  );
}
