//! Bluetooth over `bluetoothctl`'s non-interactive mode, no D-Bus
//! dependency needed, same "shell out to the standard CLI" choice as
//! Network (nmcli). None of this needs `pkexec`: BlueZ's D-Bus policy
//! already allows the active session user.
//!
//! Pairing only covers "Just Works" Simple Secure Pairing; bluetoothd's
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
    battery: Option<u32>,
    /// BlueZ device class icon ("audio-headset", "input-gaming", ...), for a type icon.
    #[serde(default)]
    icon: String,
    /// False when the device only broadcasts its address: BlueZ then uses
    /// the address itself ("7D-70-3F-22-2D-62") as the name. These are
    /// mostly beacons and phones nearby, not something you'd pair.
    named: bool,
}

/// Whether `name` is a real name rather than BlueZ's address stand-in.
fn has_real_name(name: &str, address: &str) -> bool {
    let stand_in = name.replace('-', ":");
    !name.is_empty() && !stand_in.eq_ignore_ascii_case(address)
}

#[tauri::command]
pub async fn get_adapter_powered() -> Option<bool> {
    let output = Command::new("bluetoothctl")
        .arg("show")
        .output()
        .await
        .ok()?;
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
pub async fn set_adapter_powered(on: bool) -> Result<(), String> {
    let val = if on { "on" } else { "off" };
    let out = Command::new("bluetoothctl")
        .args(["power", val])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() || get_adapter_powered().await != Some(on) {
        return Err(super::util::fail_output(&out, "Changing Bluetooth power"));
    }
    Ok(())
}

fn parse_device_lines(text: &str) -> Vec<BtDevice> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("Device ")?;
            let (addr, name) = rest.split_once(' ')?;
            let (addr, name) = (addr.trim(), name.trim());
            Some(BtDevice {
                address: addr.to_string(),
                name: name.to_string(),
                connected: false,
                battery: None,
                icon: String::new(),
                named: has_real_name(name, addr),
            })
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
    let connected: HashSet<String> = run_devices(Some("Connected"))
        .await
        .into_iter()
        .map(|d| d.address)
        .collect();
    for d in &mut devices {
        d.connected = connected.contains(&d.address);
        if let Ok(out) = Command::new("bluetoothctl")
            .args(["info", &d.address])
            .output()
            .await
        {
            let info = String::from_utf8_lossy(&out.stdout);
            d.icon = info_field(&info, "Icon:").unwrap_or_default();
            if d.connected {
                d.battery = info_field(&info, "Battery Percentage:")
                    .and_then(|v| {
                        v.split_once('(')
                            .map(|(_, v)| v.trim_end_matches(')').to_string())
                    })
                    .and_then(|v| v.parse::<u32>().ok())
                    .filter(|v| *v <= 100);
            }
        }
    }
    devices
}

fn info_field(info: &str, key: &str) -> Option<String> {
    info.lines()
        .find_map(|line| line.trim().strip_prefix(key))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Scans for a few seconds and returns every device BlueZ has seen that
/// isn't already paired.
#[tauri::command]
pub async fn scan_bluetooth() -> Vec<BtDevice> {
    let _ = Command::new("bluetoothctl")
        .args(["--timeout", "5", "scan", "on"])
        .output()
        .await;
    let paired: HashSet<String> = run_devices(Some("Paired"))
        .await
        .into_iter()
        .map(|d| d.address)
        .collect();
    let mut found: Vec<BtDevice> = run_devices(None)
        .await
        .into_iter()
        .filter(|d| !paired.contains(&d.address))
        .collect();
    // Type icons for the named ones only: there are usually a handful,
    // while unnamed beacons can number in the dozens.
    for d in found.iter_mut().filter(|d| d.named) {
        if let Ok(out) = Command::new("bluetoothctl")
            .args(["info", &d.address])
            .output()
            .await
        {
            d.icon = info_field(&String::from_utf8_lossy(&out.stdout), "Icon:").unwrap_or_default();
        }
    }
    found.sort_by_key(|d| !d.named);
    found
}

#[tauri::command]
pub async fn bt_connect(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["connect", &address])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_disconnect(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["disconnect", &address])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[tauri::command]
pub async fn bt_forget(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["remove", &address])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

/// Pairs, then confirms it actually landed in the paired-device cache:
/// bluetoothctl's exit code alone isn't a reliable signal for pairing.
#[tauri::command]
pub async fn bt_pair(address: String) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["pair", &address])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    let now_paired = run_devices(Some("Paired"))
        .await
        .iter()
        .any(|d| d.address == address);
    if now_paired {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_fields_parse() {
        let info =
            "Device AA\n\tName: Buds\n\tIcon: audio-headset\n\tBattery Percentage: 0x50 (80)\n";
        assert_eq!(info_field(info, "Icon:").as_deref(), Some("audio-headset"));
        assert_eq!(
            info_field(info, "Battery Percentage:").as_deref(),
            Some("0x50 (80)")
        );
        assert_eq!(info_field(info, "Alias:"), None);
    }

    #[test]
    fn address_stand_in_is_not_a_name() {
        let devs = parse_device_lines(
            "Device 7D:70:3F:22:2D:62 7D-70-3F-22-2D-62\nDevice E8:C9:13:88:AE:3F Galaxy Buds3\n",
        );
        assert!(!devs[0].named);
        assert!(devs[1].named);
        assert!(!has_real_name("7d-70-3f-22-2d-62", "7D:70:3F:22:2D:62"));
    }
}
