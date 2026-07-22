//! Wi-Fi + Ethernet over `nmcli`. NetworkManager lets the active session
//! user manage connections via polkit already, so the common paths (scan,
//! connect, toggle radio) need no `pkexec`. VPN import, 802.1x, and other
//! edge cases are punted to `nm-connection-editor` via the Advanced button.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::process::Command;

#[derive(Serialize, Clone)]
pub struct WifiNetwork {
    ssid: String,
    signal: i32,
    secured: bool,
    active: bool,
    known: bool,
}

async fn radio_enabled() -> bool {
    Command::new("nmcli")
        .args(["radio", "wifi"])
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "enabled")
        .unwrap_or(false)
}

async fn ethernet_status() -> Option<String> {
    let out = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE,STATE"]).arg("dev").output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find_map(|l| {
        let mut cols = l.splitn(3, ':');
        let (dev, ty, state) = (cols.next()?, cols.next()?, cols.next()?);
        (ty == "ethernet").then(|| format!("{dev}: {state}"))
    })
}

async fn known_connection_names() -> HashSet<String> {
    let Ok(out) = Command::new("nmcli").args(["-t", "-f", "NAME"]).arg("con").arg("show").output().await else {
        return HashSet::new();
    };
    String::from_utf8_lossy(&out.stdout).lines().map(str::to_string).collect()
}

#[derive(Serialize)]
pub struct NetworkInfo {
    radio_enabled: bool,
    ethernet: Option<String>,
}

#[tauri::command]
pub async fn get_network_info() -> NetworkInfo {
    NetworkInfo { radio_enabled: radio_enabled().await, ethernet: ethernet_status().await }
}

#[tauri::command]
pub async fn set_wifi_radio(enabled: bool) -> Result<(), String> {
    let val = if enabled { "on" } else { "off" };
    Command::new("nmcli").args(["radio", "wifi", val]).status().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Scan + list Wi-Fi networks, deduplicated by SSID (keeping the strongest
/// signal — the same AP shows once per band/BSSID otherwise).
#[tauri::command]
pub async fn scan_wifi() -> Vec<WifiNetwork> {
    let _ = Command::new("nmcli").args(["dev", "wifi", "rescan"]).output().await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let Ok(out) = Command::new("nmcli").args(["-t", "-f", "SSID,SIGNAL,SECURITY,IN-USE", "dev", "wifi", "list"]).output().await
    else {
        return Vec::new();
    };
    let known = known_connection_names().await;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut by_ssid: HashMap<String, WifiNetwork> = HashMap::new();
    for line in text.lines() {
        let mut cols = line.splitn(4, ':');
        let (ssid, signal, security, in_use) = (cols.next(), cols.next(), cols.next(), cols.next());
        let Some(ssid) = ssid.filter(|s| !s.is_empty()) else { continue };
        let signal: i32 = signal.and_then(|s| s.parse().ok()).unwrap_or(0);
        let net = WifiNetwork {
            ssid: ssid.to_string(),
            signal,
            secured: security.map(|s| !s.is_empty()).unwrap_or(false),
            active: in_use == Some("*"),
            known: known.contains(ssid),
        };
        by_ssid.entry(ssid.to_string()).and_modify(|existing| if net.signal > existing.signal { *existing = net.clone() }).or_insert(net);
    }
    let mut list: Vec<_> = by_ssid.into_values().collect();
    list.sort_by(|a, b| b.signal.cmp(&a.signal));
    list
}

#[tauri::command]
pub async fn connect_wifi(ssid: String, password: Option<String>) -> Result<(), String> {
    let known = known_connection_names().await;
    let output = if let Some(password) = password {
        Command::new("nmcli").args(["dev", "wifi", "connect", &ssid, "password", &password]).output().await
    } else if known.contains(&ssid) {
        Command::new("nmcli").args(["con", "up", &ssid]).output().await
    } else {
        Command::new("nmcli").args(["dev", "wifi", "connect", &ssid]).output().await
    }
    .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub fn open_connection_editor() {
    let _ = std::process::Command::new("nm-connection-editor").spawn();
}
