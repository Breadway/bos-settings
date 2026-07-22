//! hypr/autostart.json — the *extra*, user-toggleable autostart apps. The
//! core bootstrap sequence stays hardcoded in hyprland.lua on purpose (it's
//! timing/order-sensitive infrastructure, not something this panel exposes).

use serde::{Deserialize, Serialize};

use super::config;

#[derive(Clone, Serialize, Deserialize)]
pub struct AutostartEntry {
    command: String,
    #[serde(default)]
    label: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct AutostartFile {
    #[serde(default)]
    extra: Vec<AutostartEntry>,
}

fn default_extra() -> Vec<AutostartEntry> {
    vec![
        AutostartEntry { command: "breadbar".into(), label: "Bar (breadbar)".into(), enabled: true },
        AutostartEntry { command: "hypridle".into(), label: "Idle / lock daemon (hypridle)".into(), enabled: true },
        AutostartEntry { command: "bos-netcheck".into(), label: "Network connectivity check".into(), enabled: true },
        AutostartEntry { command: "breadhelp --autostart".into(), label: "BOS Help (first-run onboarding)".into(), enabled: true },
    ]
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/autostart.json")
}

#[tauri::command]
pub fn get_autostart_entries() -> Vec<AutostartEntry> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str::<AutostartFile>(&s).ok())
        .map(|f| f.extra)
        .unwrap_or_else(default_extra)
}

/// Empty-command rows (still-being-typed "Add app" entries) are dropped on
/// save rather than written as a broken autostart.json entry the Lua loader
/// would otherwise have to reject.
#[tauri::command]
pub fn save_autostart_entries(entries: Vec<AutostartEntry>) -> Result<(), String> {
    let entries: Vec<AutostartEntry> = entries.into_iter().filter(|e| !e.command.trim().is_empty()).collect();
    let file = AutostartFile { extra: entries };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}
