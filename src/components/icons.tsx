/** Minimal inline SVG icon set — consistent 16px stroke icons so the toolbar
 *  doesn't depend on Windows emoji font fallbacks. */

interface IconProps {
  size?: number;
  className?: string;
}

const base = (size: number) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
});

export const IconPlay = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polygon points="6 3 20 12 6 21 6 3" fill="currentColor" stroke="none" />
  </svg>
);

export const IconPause = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="5" y="4" width="4" height="16" fill="currentColor" stroke="none" />
    <rect x="15" y="4" width="4" height="16" fill="currentColor" stroke="none" />
  </svg>
);

export const IconPrev = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polygon points="19 4 9 12 19 20 19 4" fill="currentColor" stroke="none" />
    <line x1="5" y1="4" x2="5" y2="20" />
  </svg>
);

export const IconNext = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polygon points="5 4 15 12 5 20 5 4" fill="currentColor" stroke="none" />
    <line x1="19" y1="4" x2="19" y2="20" />
  </svg>
);

export const IconStop = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="5" y="5" width="14" height="14" rx="2" fill="currentColor" stroke="none" />
  </svg>
);

export const IconTrash = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <path d="M3 6h18" />
    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
    <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    <line x1="10" y1="11" x2="10" y2="17" />
    <line x1="14" y1="11" x2="14" y2="17" />
  </svg>
);

export const IconCopy = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="9" y="9" width="12" height="12" rx="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
);

export const IconCrop = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <path d="M6 2v14a2 2 0 0 0 2 2h14" />
    <path d="M18 22V8a2 2 0 0 0-2-2H2" />
  </svg>
);

export const IconResize = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="15 3 21 3 21 9" />
    <polyline points="9 21 3 21 3 15" />
    <line x1="21" y1="3" x2="14" y2="10" />
    <line x1="3" y1="21" x2="10" y2="14" />
  </svg>
);

export const IconUndo = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="9 14 4 9 9 4" />
    <path d="M20 20v-7a4 4 0 0 0-4-4H4" />
  </svg>
);

export const IconRedo = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="15 14 20 9 15 4" />
    <path d="M4 20v-7a4 4 0 0 1 4-4h12" />
  </svg>
);

export const IconX = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

export const IconBack = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <line x1="19" y1="12" x2="5" y2="12" />
    <polyline points="12 19 5 12 12 5" />
  </svg>
);

export const IconSave = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
    <polyline points="17 21 17 13 7 13 7 21" />
    <polyline points="7 3 7 8 15 8" />
  </svg>
);

export const IconExport = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <path d="M4 14v5a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-5" />
    <polyline points="8 7 12 3 16 7" />
    <line x1="12" y1="3" x2="12" y2="14" />
  </svg>
);

export const IconCheck = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

export const IconClock = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="12" r="9" />
    <polyline points="12 7 12 12 15 14" />
  </svg>
);

export const IconLock = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="4" y="11" width="16" height="10" rx="2" />
    <path d="M8 11V7a4 4 0 0 1 8 0v4" />
  </svg>
);

export const IconRecPlus = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <circle cx="10" cy="12" r="7" />
    <line x1="18" y1="7" x2="18" y2="13" />
    <line x1="15" y1="10" x2="21" y2="10" />
  </svg>
);

export const IconGroup = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="3" y="6" width="12" height="12" rx="2" />
    <path d="M9 6V4a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-2" />
  </svg>
);

export const IconUngroup = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <rect x="3" y="10" width="9" height="9" rx="2" />
    <path d="M8 10V7a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-3" />
    <line x1="2" y1="2" x2="22" y2="22" />
  </svg>
);

export const IconTrim = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <circle cx="6" cy="6" r="3" />
    <circle cx="6" cy="18" r="3" />
    <line x1="20" y1="4" x2="8.12" y2="15.88" />
    <line x1="14.47" y1="14.48" x2="20" y2="20" />
    <line x1="8.12" y1="8.12" x2="12" y2="12" />
  </svg>
);

export const IconMerge = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="4 7 9 12 4 17" />
    <polyline points="20 7 15 12 20 17" />
    <line x1="9" y1="12" x2="15" y2="12" />
  </svg>
);

export const IconLoop = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polyline points="17 1 21 5 17 9" />
    <path d="M3 11V9a4 4 0 0 1 4-4h14" />
    <polyline points="7 23 3 19 7 15" />
    <path d="M21 13v2a4 4 0 0 1-4 4H3" />
  </svg>
);

export const IconSpeed = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <polygon points="13 2 4 14 12 14 11 22 20 10 12 10 13 2" fill="currentColor" stroke="none" />
  </svg>
);

export const IconGear = ({ size = 16, className }: IconProps) => (
  <svg {...base(size)} className={className}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
  </svg>
);
