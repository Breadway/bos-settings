//! Display: live-connected-monitor readout (from `hyprctl monitors -j`) plus
//! an editor for `hypr/monitors.json` — the monitor *layout* Hyprland itself
//! reads at login. Like appearance.rs/autostart.rs, a plain typed struct
//! round-tripped whole (JSON has no comments to preserve).
//!
//! `Default` here (the single wildcard rule) must stay in sync with
//! `scripts/display/monitors.lua`'s own `DEFAULT_MONITORS` fallback.

use serde::{Deserialize, Serialize};

use super::config;

#[derive(Clone, Serialize, Deserialize)]
pub struct MonitorRule {
    output: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_position")]
    position: String,
    #[serde(default = "default_scale")]
    scale: String,
}

fn default_mode() -> String {
    "preferred".to_string()
}
fn default_position() -> String {
    "auto".to_string()
}
fn default_scale() -> String {
    "auto".to_string()
}

impl Default for MonitorRule {
    fn default() -> Self {
        Self { output: String::new(), mode: default_mode(), position: default_position(), scale: default_scale() }
    }
}

#[derive(Serialize, Deserialize)]
struct MonitorsFile {
    #[serde(default)]
    monitors: Vec<MonitorRule>,
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/monitors.json")
}

fn hypr_path(name: &str) -> std::path::PathBuf {
    config::config_dir().join("hypr").join(name)
}

#[derive(Serialize)]
pub struct LiveMonitor {
    name: String,
    mode: String,
}

#[tauri::command]
pub fn get_live_monitors() -> Vec<LiveMonitor> {
    let Some(value) = bread_utils::proc::run_json("hyprctl", &["monitors", "-j"], std::time::Duration::from_secs(3))
    else {
        return Vec::new();
    };
    let Ok(monitors) = serde_json::from_value::<Vec<serde_json::Value>>(value) else {
        return Vec::new();
    };
    monitors
        .iter()
        .filter_map(|m| {
            let name = m.get("name")?.as_str()?;
            let w = m.get("width")?.as_u64()?;
            let h = m.get("height")?.as_u64()?;
            let refresh = m.get("refreshRate")?.as_f64()?;
            Some(LiveMonitor { name: name.to_string(), mode: format!("{w}x{h} @ {refresh:.0}Hz") })
        })
        .collect()
}

#[tauri::command]
pub fn get_monitor_rules() -> Vec<MonitorRule> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str::<MonitorsFile>(&s).ok())
        .filter(|f| !f.monitors.is_empty())
        .map(|f| f.monitors)
        .unwrap_or_else(|| vec![MonitorRule::default()])
}

#[tauri::command]
pub fn save_monitor_rules(rules: Vec<MonitorRule>) -> Result<(), String> {
    let file = MonitorsFile { monitors: rules };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}

/// Opens `hyprland.lua` in `$EDITOR` (nano if unset) inside a terminal —
/// spawning a TUI editor with no terminal to attach to is a silent no-op.
#[tauri::command]
pub fn open_hyprland_conf() {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let path = hypr_path("hyprland.lua");
    let _ = std::process::Command::new("kitty").args(["-e", &editor]).arg(path).spawn();
}

#[tauri::command]
pub fn open_keybinds_viewer() {
    let _ = std::process::Command::new("breadhelp").spawn();
}
