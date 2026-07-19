import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import App from "./App";
import RecorderPanel from "./views/RecorderPanel";
import { isTauri } from "./lib/ipc";
import { applyTheme, cachedTheme } from "./lib/theme";

// One frontend bundle serves every window kind; windows are routed by their
// Tauri label ("main", "recorder").
async function windowKind(): Promise<string> {
  if (!isTauri) return "main";
  const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
  return getCurrentWebviewWindow().label;
}

const root = ReactDOM.createRoot(document.getElementById("root")!);

void windowKind().then((label) => {
  // The recorder frame is a transparent OS window; the opaque app body would
  // paint it solid black, so opt it into a clear background. It also stays
  // dark-chromed regardless of the app theme (it floats over arbitrary screen
  // content), so it never applies the cached light theme.
  if (label === "recorder") {
    document.documentElement.classList.add("transparent-window");
  } else {
    // Main window: apply the last-used theme before first paint to avoid a
    // dark→light flash; App reconciles it with the persisted settings on boot.
    const cached = cachedTheme();
    if (cached) applyTheme(cached);
  }
  root.render(
    <React.StrictMode>
      {label === "recorder" ? <RecorderPanel /> : <App />}
    </React.StrictMode>,
  );
});
