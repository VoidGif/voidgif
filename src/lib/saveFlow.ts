import { save } from "@tauri-apps/plugin-dialog";
import { saveProject } from "./ipc";
import { dirOf, joinDir } from "./lastDir";
import { useAppStore } from "../stores/appStore";
import { useSettingsStore } from "../stores/settingsStore";

/**
 * Opens the OS save dialog and writes the current session to a .voidgif file.
 * Clears the store's `unsaved` flag on success. `onWriteStart` fires after the
 * user picks a path, right before the (potentially slow) write. The dialog
 * reopens at the folder of the last project save.
 */
export async function promptSaveProject(
  onWriteStart?: () => void,
): Promise<"saved" | "cancelled" | "error"> {
  try {
    const lastDir = useSettingsStore.getState().lastProjectDir;
    const path = await save({
      defaultPath: lastDir ? joinDir(lastDir, "recording.voidgif") : "recording.voidgif",
      filters: [{ name: "VoidGif project", extensions: ["voidgif"] }],
    });
    if (!path) return "cancelled";
    onWriteStart?.();
    await saveProject(path);
    const dir = dirOf(path);
    if (dir) useSettingsStore.getState().update({ lastProjectDir: dir });
    useAppStore.getState().markSaved();
    return "saved";
  } catch {
    return "error";
  }
}
