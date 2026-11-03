//! Wi-Fi + Ethernet over `nmcli`. NetworkManager lets the active session
//! user manage connections via polkit already, so the common paths (scan,
//! connect, toggle radio) need no `pkexec`. VPN import, 802.1x, and other
//! edge cases are punted to `nm-connection-editor` via the Advanced button.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::process::Command;

use super::util;

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
    let out = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE"])
        .arg("dev")
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().find_map(|l| {
        let mut cols = l.splitn(3, ':');
        let (dev, ty, state) = (cols.next()?, cols.next()?, cols.next()?);
        (ty == "ethernet").then(|| format!("{dev}: {state}"))
    })
}

async fn known_connection_names() -> HashSet<String> {
    let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "NAME"])
        .arg("con")
        .arg("show")
        .output()
        .await
    else {
        return HashSet::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

#[derive(Serialize)]
pub struct NetworkInfo {
    radio_enabled: bool,
    ethernet: Option<String>,
    ssid: Option<String>,
}

/// The SSID NetworkManager reports as the active connection, if any. This is
/// the authoritative answer for "am I connected"; the scan list below can miss
/// the access point the device is on.
async fn connected_ssid() -> Option<String> {
    Command::new("nmcli")
        .args([
            "-t",
            "--escape",
            "no",
            "-f",
            "ACTIVE,SSID",
            "device",
            "wifi",
            "list",
            "--rescan",
            "no",
        ])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|line| line.strip_prefix("yes:").map(String::from))
        })
}

#[tauri::command]
pub async fn get_network_info() -> NetworkInfo {
    NetworkInfo {
        radio_enabled: radio_enabled().await,
        ethernet: ethernet_status().await,
        ssid: connected_ssid().await,
    }
}

#[tauri::command]
pub async fn set_wifi_radio(enabled: bool) -> Result<(), String> {
    let val = if enabled { "on" } else { "off" };
    Command::new("nmcli")
        .args(["radio", "wifi", val])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Scan + list Wi-Fi networks, one row per SSID.
///
/// The same SSID shows once per band and BSSID, so a school or mesh network
/// appears several times with different signals. The associated access point
/// wins; otherwise the strongest signal is kept. `active` is kept if any of
/// the rows for that SSID is in use: keeping only the strongest row used to
/// drop `IN-USE` whenever the device was on a weaker access point, and the
/// page then said "Not connected" while the connection was fine.
#[tauri::command]
pub async fn scan_wifi() -> Vec<WifiNetwork> {
    let _ = Command::new("nmcli")
        .args(["dev", "wifi", "rescan"])
        .output()
        .await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let Ok(out) = Command::new("nmcli")
        .args([
            "-t",
            "--escape",
            "no",
            // SSID goes last: it is the only field that can contain a colon,
            // and `--escape no` leaves those colons raw.
            "-f",
            "IN-USE,SIGNAL,SECURITY,SSID",
            "dev",
            "wifi",
            "list",
        ])
        .output()
        .await
    else {
        return Vec::new();
    };
    let known = known_connection_names().await;
    let mut list = parse_wifi_list(&String::from_utf8_lossy(&out.stdout), &known);
    if let Some(ssid) = connected_ssid().await {
        for net in &mut list {
            if net.ssid == ssid {
                net.active = true;
            }
        }
    }
    list
}

fn parse_wifi_list(text: &str, known: &HashSet<String>) -> Vec<WifiNetwork> {
    let mut by_ssid: HashMap<String, WifiNetwork> = HashMap::new();
    for line in text.lines() {
        let mut cols = line.splitn(4, ':');
        let (Some(in_use), Some(signal), Some(security), Some(ssid)) =
            (cols.next(), cols.next(), cols.next(), cols.next())
        else {
            continue;
        };
        if ssid.is_empty() {
            continue;
        }
        let net = WifiNetwork {
            ssid: ssid.to_string(),
            signal: signal.parse().unwrap_or(0),
            secured: !security.is_empty(),
            active: in_use == "*",
            known: known.contains(ssid),
        };
        match by_ssid.entry(net.ssid.clone()) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(net);
            }
            std::collections::hash_map::Entry::Occupied(mut slot) => {
                let existing = slot.get();
                let active = existing.active || net.active;
                let replace = net.active || (!existing.active && net.signal > existing.signal);
                if replace {
                    *slot.get_mut() = net;
                }
                slot.get_mut().active = active;
            }
        }
    }
    let mut list: Vec<_> = by_ssid.into_values().collect();
    list.sort_by_key(|n| std::cmp::Reverse(n.signal));
    list
}

#[tauri::command]
pub async fn connect_wifi(ssid: String, password: Option<String>) -> Result<(), String> {
    if !util::valid_nm_id(&ssid) {
        return Err(format!("invalid SSID '{ssid}'"));
    }
    let known = known_connection_names().await;
    let output = if let Some(password) = password {
        Command::new("nmcli")
            .args(["dev", "wifi", "connect", &ssid, "password", &password])
            .output()
            .await
    } else if known.contains(&ssid) {
        Command::new("nmcli")
            .args(["con", "up", &ssid])
            .output()
            .await
    } else {
        Command::new("nmcli")
            .args(["dev", "wifi", "connect", &ssid])
            .output()
            .await
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

#[cfg(test)]
mod tests {
    use super::*;

    fn known(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn active_access_point_survives_a_stronger_duplicate() {
        // A school network lists one row per access point. The device is on the
        // 70-signal AP while another NCC AP is stronger.
        let text = " :90:WPA2:Totally NCC\n*:70:WPA2:NCC\n :82:WPA2:NCC\n";
        let list = parse_wifi_list(text, &known(&["NCC"]));
        assert_eq!(list.len(), 2);
        let ncc = list.iter().find(|n| n.ssid == "NCC").unwrap();
        assert!(ncc.active);
        assert!(ncc.known);
        assert_eq!(ncc.signal, 70);
    }

    #[test]
    fn ssid_with_a_colon_is_kept_whole() {
        let text = "*:80:WPA2:Lab: guest\n";
        let list = parse_wifi_list(text, &HashSet::new());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].ssid, "Lab: guest");
        assert_eq!(list[0].signal, 80);
        assert!(list[0].secured);
        assert!(list[0].active);
    }

    #[test]
    fn empty_ssid_rows_are_skipped() {
        let text = ":25:WPA2:\n*:80:WPA2:Home\n";
        let list = parse_wifi_list(text, &HashSet::new());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].ssid, "Home");
    }
}
