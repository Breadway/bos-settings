//! Accessibility toggles that actually do something on Hyprland.
//! Orca launches. Magnifier is Hyprland `cursor:zoom_factor`. Sticky/slow
//! keys are not exposed by Hyprland or xkeyboard-config rules — the UI
//! must show that honestly rather than a dead switch.

use serde::Serialize;
use tokio::process::Command;

use super::util::{command_exists, fail_output, pacman_installed};

#[derive(Serialize)]
pub struct A11yStatus {
    orca_installed: bool,
    orca_running: bool,
    zoom_factor: f64,
    sticky_keys_supported: bool,
    slow_keys_supported: bool,
    kmag_installed: bool,
    note: String,
}

async fn orca_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "orca"])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn read_zoom() -> f64 {
    let output = Command::new("hyprctl")
        .args(["getoption", "cursor:zoom_factor", "-j"])
        .output()
        .await;
    let Ok(output) = output else {
        return 1.0;
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return 1.0;
    };
    v.get("float")
        .and_then(|x| x.as_f64())
        .or_else(|| v.get("int").and_then(|x| x.as_i64()).map(|i| i as f64))
        .unwrap_or(1.0)
}

#[tauri::command]
pub async fn get_a11y_status() -> A11yStatus {
    A11yStatus {
        orca_installed: command_exists("orca") || pacman_installed("orca"),
        orca_running: orca_running().await,
        zoom_factor: read_zoom().await,
        sticky_keys_supported: false,
        slow_keys_supported: false,
        kmag_installed: command_exists("kmag") || pacman_installed("kmag"),
        note: "Hyprland does not expose XKB AccessX (sticky keys / slow keys). Those toggles stay off because they would not do anything.".into(),
    }
}

#[tauri::command]
pub async fn set_cursor_zoom(factor: f64) -> Result<f64, String> {
    let factor = factor.clamp(1.0, 8.0);
    let value = format!("{factor:.2}");
    let output = Command::new("hyprctl")
        .args(["keyword", "cursor:zoom_factor", &value])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(factor)
    } else {
        Err(fail_output(&output, "hyprctl keyword cursor:zoom_factor"))
    }
}

#[tauri::command]
pub async fn set_orca_running(running: bool) -> Result<(), String> {
    if running {
        if !command_exists("orca") {
            return Err("orca is not installed".into());
        }
        std::process::Command::new("orca")
            .arg("--replace")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("couldn't start orca: {e}"))?;
        Ok(())
    } else {
        let _ = Command::new("pkill").args(["-x", "orca"]).status().await;
        Ok(())
    }
}

#[tauri::command]
pub fn open_kmag() -> Result<(), String> {
    if !command_exists("kmag") {
        return Err("kmag is not installed".into());
    }
    std::process::Command::new("kmag")
        .spawn()
        .map_err(|e| format!("couldn't start kmag: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn zoom_clamp_bounds() {
        let f = 0.2_f64.clamp(1.0, 8.0);
        assert_eq!(f, 1.0);
        assert_eq!(12.0_f64.clamp(1.0, 8.0), 8.0);
    }
}
