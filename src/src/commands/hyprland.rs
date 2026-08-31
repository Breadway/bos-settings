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

#[derive(Serialize, Deserialize, Clone)]
pub struct LiveMonitor {
    pub name: String,
    pub mode: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub scale: f64,
    pub transform: u32,
    pub available_modes: Vec<String>,
}

fn json_i32(v: &serde_json::Value, key: &str) -> Option<i32> {
    v.get(key)?.as_i64().map(|n| n as i32).or_else(|| v.get(key)?.as_f64().map(|n| n.round() as i32))
}

fn json_u32(v: &serde_json::Value, key: &str) -> Option<u32> {
    v.get(key)?.as_u64().map(|n| n as u32).or_else(|| v.get(key)?.as_f64().map(|n| n.round() as u32))
}

fn json_f64(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key)?.as_f64().or_else(|| v.get(key)?.as_u64().map(|n| n as f64))
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
            let name = m.get("name")?.as_str()?.to_string();
            let width = json_u32(m, "width")?;
            let height = json_u32(m, "height")?;
            let refresh = json_f64(m, "refreshRate").unwrap_or(60.0);
            let x = json_i32(m, "x").unwrap_or(0);
            let y = json_i32(m, "y").unwrap_or(0);
            let scale = json_f64(m, "scale").unwrap_or(1.0).max(0.1);
            let transform = json_u32(m, "transform").unwrap_or(0);
            let available_modes = m
                .get("availableModes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(LiveMonitor {
                name,
                mode: format!("{width}x{height} @ {refresh:.0}Hz"),
                x,
                y,
                width,
                height,
                refresh,
                scale,
                transform,
                available_modes,
            })
        })
        .collect()
}

fn write_monitor_rules(rules: &[MonitorRule]) -> Result<(), String> {
    let file = MonitorsFile { monitors: rules.to_vec() };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}

fn lua_ident(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!("bad monitor name '{name}'"));
    }
    Ok(())
}

/// Live-apply a layout via BOS Hyprland `hl.monitor()`, then persist monitors.json.
#[tauri::command]
pub fn apply_monitor_layout(monitors: Vec<LiveMonitor>) -> Result<(), String> {
    if monitors.is_empty() {
        return Err("no monitors".into());
    }
    let mut stmts = Vec::new();
    let mut rules = Vec::new();
    for m in &monitors {
        lua_ident(&m.name)?;
        let refresh = if m.refresh > 1.0 { m.refresh.round() as u32 } else { 60 };
        let mode = format!("{}x{}@{refresh}", m.width.max(1), m.height.max(1));
        let position = format!("{}x{}", m.x, m.y);
        let scale = if (m.scale - 1.0).abs() < 0.001 {
            "1".to_string()
        } else {
            format!("{:.2}", m.scale)
        };
        let transform = m.transform.min(7);
        stmts.push(format!(
            "hl.monitor({{ output = \"{}\", mode = \"{mode}\", position = \"{position}\", scale = \"{scale}\", transform = {transform}, vrr = false }})",
            m.name
        ));
        rules.push(MonitorRule {
            output: m.name.clone(),
            mode,
            position,
            scale,
        });
    }
    let lua = stmts.join("; ");
    let output = std::process::Command::new("hyprctl")
        .args(["eval", &lua])
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout != "ok" {
        let low = stdout.to_lowercase();
        if low.contains("unknown request") || low.contains("unknown command") || stdout.is_empty() {
            return Err("Live layout needs BOS Hyprland (hyprctl eval / hl.monitor).".into());
        }
        return Err(format!("hyprctl: {stdout}"));
    }
    write_monitor_rules(&rules)?;
    Ok(())
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
    write_monitor_rules(&rules)?;
    super::util::hypr_reload();
    Ok(())
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
