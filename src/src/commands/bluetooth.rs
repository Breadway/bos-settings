//! Bluetooth over `bluetoothctl`'s non-interactive mode — no D-Bus
//! dependency needed, same "shell out to the standard CLI" choice as
//! Network (nmcli). None of this needs `pkexec` — BlueZ's D-Bus policy
//! already allows the active session user.
//!
//! Pairing only covers "Just Works" Simple Secure Pairing — bluetoothd's
//! own built-in default agent auto-accepts that for most audio/HID
//! devices. A device that requires PIN/passkey confirmation isn't
//! supported (would need this app to register its own bluetoothd agent);
//! pairing such a device just fails, surfaced as an error.

use serde::Serialize;
use std::collections::HashSet;
use tokio::process::Command;

#[derive(Serialize, Clone)]
pub struct BtDevice {
    address: String,
    name: String,
    connected: bool,
}

#[tauri::command]
pub async fn get_adapter_powered() -> Option<bool> {
    let output = Command::new("bluetoothctl").arg("show").output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    if !text.trim_start().starts_with("Controller") {
        return None;
    }
    Some(text.lines().any(|l| l.trim() == "Powered: yes"))
}

#[tauri::command]
pub async fn set_adapter_powered(on: bool) {
    let val = if on { "on" } else { "off" };
    let _ = Command::new("bluetoothctl").args(["power", val]).status().await;
}

fn parse_device_lines(text: &str) -> Vec<BtDevice> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("Device ")?;
            let (addr, name) = rest.split_once(' ')?;
            Some(BtDevice { address: addr.trim().to_string(), name: name.trim().to_string(), connected: false })
        })
        .collect()
}

async fn run_devices(filter: Option<&str>) -> Vec<BtDevice> {
    let mut args = vec!["devices"];
    if let Some(f) = filter {
        args.push(f);
    }
    let Ok(out) = Command::new("bluetoothctl").args(&args).output().await else {
        return Vec::new();
    };
    parse_device_lines(&String::from_utf8_lossy(&out.stdout))
}

#[tauri::command]
pub async fn get_paired_devices() -> Vec<BtDevice> {
    let mut devices = run_devices(Some("Paired")).await;
    let connected: HashSet<String> = run_devices(Some("Connected")).await.into_iter().map(|d| d.address).collect();
    for d in &mut devices {
        d.connected = connected.contains(&d.address);
    }
    devices
}

/// Scans for a few seconds and returns every device BlueZ has seen that
/// isn't already paired.
#[tauri::command]
pub async fn scan_bluetooth() -> Vec<BtDevice> {
    let _ = Command::new("bluetoothctl").args(["--timeout", "5", "scan", "on"]).output().await;
    let paired: HashSet<String> = run_devices(Some("Paired")).await.into_iter().map(|d| d.address).collect();
    run_devices(None).await.into_iter().filter(|d| !paired.contains(&d.address)).collect()
}

#[tauri::command]
pub async fn bt_connect(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl").args(["connect", &address]).output().await.map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_disconnect(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl").args(["disconnect", &address]).output().await.map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_forget(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl").args(["remove", &address]).output().await.map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// Pairs, then confirms it actually landed in the paired-device cache —
/// bluetoothctl's exit code alone isn't a reliable signal for pairing.
#[tauri::command]
pub async fn bt_pair(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl").args(["pair", &address]).output().await.map_err(|e| e.to_string())?;
    let now_paired = run_devices(Some("Paired")).await.iter().any(|d| d.address == address);
    if now_paired {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}
