//! Night light via hyprsunset (Hyprland twilight IPC). The compositor
//! talks to a hyprsunset daemon socket; if the binary is missing we offer
//! a pacman install rather than pretending the toggle works.

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::config;
use super::util::{self, command_exists, fail_output};

const FRAGMENT: &str = "nightlight.conf";
const DEFAULT_TEMP: u32 = 3500;
const MIN_TEMP: u32 = 2000;
const MAX_TEMP: u32 = 6500;

#[derive(Serialize, Deserialize, Clone)]
pub struct NightlightConfig {
    enabled: bool,
    temperature: u32,
}

impl Default for NightlightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: DEFAULT_TEMP,
        }
    }
}

#[derive(Serialize)]
pub struct NightlightStatus {
    installed: bool,
    running: bool,
    enabled: bool,
    temperature: u32,
    error: Option<String>,
}

fn persist_path() -> std::path::PathBuf {
    util::bos_settings_dir().join("nightlight.toml")
}

fn load_persist() -> NightlightConfig {
    let Ok(text) = std::fs::read_to_string(persist_path()) else {
        return NightlightConfig::default();
    };
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    NightlightConfig {
        enabled: config::get_bool(&doc, &["enabled"]).unwrap_or(false),
        temperature: config::get_i64(&doc, &["temperature"])
            .unwrap_or(DEFAULT_TEMP as i64)
            .clamp(MIN_TEMP as i64, MAX_TEMP as i64) as u32,
    }
}

fn save_persist(cfg: &NightlightConfig) -> Result<(), String> {
    let mut doc = toml_edit::DocumentMut::new();
    config::set_bool(&mut doc, &["enabled"], cfg.enabled);
    config::set_i64(&mut doc, &["temperature"], cfg.temperature as i64);
    let path = persist_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    config::atomic_write(&path, &doc.to_string()).map_err(|e| e.to_string())
}

fn clamp_temp(t: u32) -> u32 {
    t.clamp(MIN_TEMP, MAX_TEMP)
}

async fn hyprsunset_running() -> bool {
    Command::new("hyprctl")
        .args(["hyprsunset", "gamma", "1.0"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn start_daemon() -> Result<(), String> {
    if hyprsunset_running().await {
        return Ok(());
    }
    if !command_exists("hyprsunset") {
        return Err("hyprsunset is not installed".into());
    }
    std::process::Command::new("hyprsunset")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("couldn't start hyprsunset: {e}"))?;
    for _ in 0..15 {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if hyprsunset_running().await {
            return Ok(());
        }
    }
    Err("hyprsunset started but Hyprland twilight socket never came up".into())
}

async fn apply_temperature(temp: u32) -> Result<(), String> {
    start_daemon().await?;
    let t = clamp_temp(temp).to_string();
    let output = Command::new("hyprctl")
        .args(["hyprsunset", "temperature", &t])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "hyprctl hyprsunset"))
    }
}

async fn apply_identity() -> Result<(), String> {
    if !hyprsunset_running().await {
        return Ok(());
    }
    let output = Command::new("hyprctl")
        .args(["hyprsunset", "identity"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "hyprctl hyprsunset"))
    }
}

fn write_autostart() -> Result<(), String> {
    let path = util::hypr_dir().join(FRAGMENT);
    std::fs::create_dir_all(util::hypr_dir()).map_err(|e| e.to_string())?;
    config::atomic_write(&path, "exec-once = hyprsunset\n").map_err(|e| e.to_string())?;
    util::ensure_hypr_source(FRAGMENT)
}

fn clear_autostart() -> Result<(), String> {
    let path = util::hypr_dir().join(FRAGMENT);
    let _ = std::fs::remove_file(path);
    util::remove_hypr_source(FRAGMENT)
}

#[tauri::command]
pub async fn get_nightlight() -> NightlightStatus {
    let persist = load_persist();
    let installed = command_exists("hyprsunset");
    let running = if installed {
        hyprsunset_running().await
    } else {
        false
    };
    NightlightStatus {
        installed,
        running,
        enabled: persist.enabled && running,
        temperature: persist.temperature,
        error: None,
    }
}

#[tauri::command]
pub async fn set_nightlight(enabled: bool, temperature: u32) -> Result<NightlightStatus, String> {
    if !command_exists("hyprsunset") {
        return Ok(NightlightStatus {
            installed: false,
            running: false,
            enabled: false,
            temperature: clamp_temp(temperature),
            error: Some("hyprsunset is not installed".into()),
        });
    }
    let mut cfg = NightlightConfig {
        enabled,
        temperature: clamp_temp(temperature),
    };
    let mut error = None;
    if enabled {
        if let Err(e) = apply_temperature(cfg.temperature).await {
            error = Some(e);
            cfg.enabled = false;
        } else if let Err(e) = write_autostart() {
            error = Some(e);
        }
    } else {
        if let Err(e) = apply_identity().await {
            error = Some(e);
        }
        if let Err(e) = clear_autostart() {
            error = Some(error.unwrap_or(e));
        }
    }
    save_persist(&cfg)?;
    Ok(NightlightStatus {
        installed: true,
        running: hyprsunset_running().await,
        enabled: cfg.enabled,
        temperature: cfg.temperature,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_clamps() {
        assert_eq!(clamp_temp(100), MIN_TEMP);
        assert_eq!(clamp_temp(9000), MAX_TEMP);
        assert_eq!(clamp_temp(3500), 3500);
    }
}
