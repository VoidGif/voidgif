//! Persisted user settings: theme, language, and capture defaults.
//!
//! Stored as JSON alongside the recorder-window geometry in the app config
//! dir. A missing OR corrupt file loads as `None` (not `Default`) — the
//! frontend treats "no settings file" as first run and shows onboarding, so a
//! garbled write must not silently skip it.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// "dark" | "light"
    pub theme: String,
    /// "ko" | "ja" | "en"; `None` = follow the OS language.
    pub language: Option<String>,
    /// Default capture rate seeded into the recorder (1..=60).
    pub default_fps: u32,
    /// Whether the cursor is captured by default.
    pub default_cursor: bool,
    /// Folder the last export was written to; the export dialog reopens here.
    /// `#[serde(default)]` so a settings file written before this field existed
    /// still deserializes — a missing field must NOT make `load` return `None`
    /// and re-trigger onboarding. Not validated: any path string is accepted.
    #[serde(default)]
    pub last_export_dir: Option<String>,
    /// Folder the last `.voidgif` was saved to; the save dialog reopens here.
    #[serde(default)]
    pub last_project_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: None,
            default_fps: 30,
            default_cursor: true,
            last_export_dir: None,
            last_project_dir: None,
        }
    }
}

fn settings_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

/// Loads settings from disk. `None` when the file is missing, unreadable, or
/// malformed — the caller uses that to decide whether onboarding runs.
pub fn load(app: &AppHandle) -> Option<Settings> {
    let raw = std::fs::read_to_string(settings_path(app)?).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Rejects out-of-range/unknown values so a hand-edited or buggy write can't
/// persist a state the frontend can't render.
fn validate(s: &Settings) -> Result<(), String> {
    if s.theme != "dark" && s.theme != "light" {
        return Err(format!("invalid theme: {}", s.theme));
    }
    if let Some(lang) = &s.language {
        if !matches!(lang.as_str(), "ko" | "ja" | "en") {
            return Err(format!("invalid language: {lang}"));
        }
    }
    if !(1..=60).contains(&s.default_fps) {
        return Err(format!("default_fps out of range (1..=60): {}", s.default_fps));
    }
    Ok(())
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    validate(settings)?;
    let path = settings_path(app).ok_or("no app config dir")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// `None` when no settings file exists yet (first run → onboarding).
#[tauri::command]
pub fn get_settings(app: AppHandle) -> Option<Settings> {
    load(&app)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    save(&app, &settings)
}
