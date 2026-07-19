/**
 * Last-used-folder helpers for the save/export dialogs. Pure string ops — no
 * `@tauri-apps/api/path` — so remembering a folder needs no extra filesystem
 * capability/ACL surface. Dialog paths are already absolute and OS-native.
 */

/**
 * Parent directory of an absolute file path, or `""` when there's no separator
 * to split on (the caller then stores nothing rather than an empty folder).
 */
export function dirOf(filePath: string): string {
  const i = Math.max(filePath.lastIndexOf("\\"), filePath.lastIndexOf("/"));
  return i > 0 ? filePath.slice(0, i) : "";
}

/**
 * Joins a directory and file name with whichever separator the directory
 * already uses (Windows dialogs hand back `\`, POSIX `/`).
 */
export function joinDir(dir: string, name: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  return `${dir}${sep}${name}`;
}
