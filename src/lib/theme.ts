import type { ThemeName } from "../types";

const THEME_KEY = "vg-theme";

/**
 * Applies a theme by toggling the "light" class on <html> (dark = no class,
 * matching the CSS-variable overrides in styles.css). Also caches the choice
 * so main.tsx can apply it before first paint and avoid a dark→light flash on
 * reload. The recorder window never calls this, so it always stays dark.
 */
export function applyTheme(theme: ThemeName): void {
  document.documentElement.classList.toggle("light", theme === "light");
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Private-mode / storage-disabled: theme still applies for this session.
  }
}

/** The last theme applied on this machine, if any (used for flash-free boot). */
export function cachedTheme(): ThemeName | null {
  try {
    const v = localStorage.getItem(THEME_KEY);
    return v === "light" || v === "dark" ? v : null;
  } catch {
    return null;
  }
}
