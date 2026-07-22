//! Output/input volume and device selection over PipeWire's pulse
//! compatibility layer (`pactl`) — the same surface hyprland.lua's media
//! keys already use via `wpctl`. `pactl` is used here instead because it
//! can enumerate devices with human-readable descriptions and switch the
//! default in one command; `wpctl` cannot easily do either.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command;

#[derive(Deserialize, Serialize, Clone)]
pub struct SoundDevice {
    name: String,
    description: String,
    mute: bool,
    percent: f64,
}

#[derive(Deserialize)]
struct RawDevice {
    name: String,
    description: String,
    mute: bool,
    volume: HashMap<String, RawVolumeChannel>,
}

#[derive(Deserialize)]
struct RawVolumeChannel {
    value_percent: String,
}

impl RawDevice {
    fn percent(&self) -> f64 {
        self.volume
            .values()
            .next()
            .and_then(|v| v.value_percent.trim_end_matches('%').trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}

async fn list_devices(kind: &str) -> Vec<SoundDevice> {
    let Ok(output) = Command::new("pactl").args(["-f", "json", "list", kind]).output().await else {
        return Vec::new();
    };
    let raw: Vec<RawDevice> = serde_json::from_slice(&output.stdout).unwrap_or_default();
    raw.into_iter()
        .map(|d| SoundDevice { name: d.name.clone(), description: d.description.clone(), mute: d.mute, percent: d.percent() })
        .collect()
}

async fn default_device_name(kind: &str) -> Option<String> {
    let flag = if kind == "sinks" { "get-default-sink" } else { "get-default-source" };
    Command::new("pactl")
        .arg(flag)
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Serialize)]
pub struct SoundSection {
    devices: Vec<SoundDevice>,
    default_name: Option<String>,
}

#[tauri::command]
pub async fn get_sound_section(kind: String) -> SoundSection {
    let devices = list_devices(&kind).await;
    let default_name = default_device_name(&kind).await;
    SoundSection { devices, default_name }
}

#[tauri::command]
pub async fn set_default_sound_device(kind: String, name: String) -> Result<(), String> {
    let flag = if kind == "sinks" { "set-default-sink" } else { "set-default-source" };
    Command::new("pactl").args([flag, &name]).status().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_sound_volume(kind: String, name: String, percent: i64) -> Result<(), String> {
    let flag = if kind == "sinks" { "set-sink-volume" } else { "set-source-volume" };
    let pct = format!("{percent}%");
    Command::new("pactl").args([flag, &name, &pct]).status().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_sound_mute(kind: String, name: String, mute: bool) -> Result<(), String> {
    let flag = if kind == "sinks" { "set-sink-mute" } else { "set-source-mute" };
    let val = if mute { "1" } else { "0" };
    Command::new("pactl").args([flag, &name, val]).status().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_mixer() {
    let _ = std::process::Command::new("pavucontrol").spawn();
}
