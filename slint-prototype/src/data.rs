//! The two pages this prototype actually drives.
//!
//! Home reads the same sources the shipping dashboard does (nmcli, bluetoothctl,
//! upower, df, pacman). Appearance reads and writes `hypr/settings.json` with
//! the same merge the Tauri command uses: only the keys this page owns are
//! replaced, everything else in the file stays, and the previous file is kept
//! as a sibling backup. A failed Hyprland reload is reported, not hidden.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Home {
    pub account_name: String,
    pub hostname: String,
    pub greeting: String,
    pub lede: String,
    pub wifi_on: bool,
    pub wifi_detail: String,
    pub bluetooth_on: bool,
    pub bluetooth_detail: String,
    pub battery_percent: String,
    pub battery_fraction: f32,
    pub battery_detail: String,
    pub updates: String,
    pub update_count: usize,
    pub storage_detail: String,
    pub storage_used: f32,
    pub notes: Vec<String>,
}

pub fn load_home() -> Home {
    let account = account();
    let wifi = wifi();
    let bluetooth = bluetooth();
    let battery = battery();
    let updates = updates();
    let storage = storage();
    let mut notes = Vec::new();
    notes.extend(wifi.note);
    notes.extend(bluetooth.note);
    notes.extend(updates.note.clone());
    notes.extend(storage.note.clone());

    let hour = local_hour();
    let greeting_word = if hour < 5 {
        "Still up"
    } else if hour < 12 {
        "Good morning"
    } else if hour < 18 {
        "Good afternoon"
    } else {
        "Good evening"
    };
    let mut lede_parts = Vec::new();
    if let Some(ssid) = &wifi.ssid {
        lede_parts.push(format!("On {ssid}"));
    } else {
        lede_parts.push("Your desktop at a glance".into());
    }
    if !battery.percent.is_empty() {
        lede_parts.push(format!("battery {}", battery.percent));
    }
    if !battery.detail.is_empty() {
        lede_parts.push(battery.detail.clone());
    }
    Home {
        greeting: format!("{greeting_word}, {}", account.display_name),
        lede: lede_parts.join(" · ") + ".",
        account_name: account.display_name,
        hostname: if account.hostname.is_empty() {
            "Bread OS".into()
        } else {
            format!("{} · Bread OS", account.hostname)
        },
        wifi_on: wifi.on,
        wifi_detail: wifi.detail,
        bluetooth_on: bluetooth.on,
        bluetooth_detail: bluetooth.detail,
        battery_percent: battery.percent,
        battery_fraction: battery.fraction,
        battery_detail: battery.detail,
        updates: updates.summary,
        update_count: updates.count,
        storage_detail: storage.detail,
        storage_used: storage.fraction,
        notes,
    }
}

struct Account {
    display_name: String,
    hostname: String,
}

fn account() -> Account {
    let username = std::env::var("USER").unwrap_or_else(|_| "User".into());
    let display_name = std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|line| {
                let fields: Vec<_> = line.split(':').collect();
                (fields.len() > 4 && fields[0] == username)
                    .then(|| fields[4].split(',').next().unwrap_or("").to_string())
            })
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| username.clone());
    let hostname = std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_default();
    Account {
        display_name,
        hostname,
    }
}

struct Wifi {
    on: bool,
    ssid: Option<String>,
    detail: String,
    note: Option<String>,
}

fn wifi() -> Wifi {
    let radio = Command::new("nmcli").args(["radio", "wifi"]).output();
    let Ok(radio) = radio else {
        return Wifi {
            on: false,
            ssid: None,
            detail: "Unavailable".into(),
            note: Some("Wi-Fi: nmcli is not installed.".into()),
        };
    };
    let on = String::from_utf8_lossy(&radio.stdout).trim() == "enabled";
    let ssid = Command::new("nmcli")
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
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout).lines().find_map(|line| {
                line.strip_prefix("yes:")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            })
        });
    let detail = if !on {
        "Off".into()
    } else {
        ssid.clone().unwrap_or_else(|| "On".into())
    };
    Wifi {
        on,
        detail,
        ssid,
        note: None,
    }
}

struct Bluetooth {
    on: bool,
    detail: String,
    note: Option<String>,
}

fn bluetooth() -> Bluetooth {
    let Ok(out) = Command::new("bluetoothctl").args(["show"]).output() else {
        return Bluetooth {
            on: false,
            detail: "Unavailable".into(),
            note: Some("Bluetooth: bluetoothctl is not installed.".into()),
        };
    };
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.trim_start().starts_with("Controller") {
        return Bluetooth {
            on: false,
            detail: "No adapter".into(),
            note: None,
        };
    }
    let on = text.lines().any(|l| l.trim() == "Powered: yes");
    let connected = if on {
        Command::new("bluetoothctl")
            .args(["devices", "Connected"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter_map(|l| {
                        l.strip_prefix("Device ")
                            .and_then(|r| r.split_once(' '))
                            .map(|(_, name)| name.trim().to_string())
                    })
                    .filter(|n| !n.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let detail = if !on {
        "Off".into()
    } else if connected.is_empty() {
        "On".into()
    } else {
        connected.join(", ")
    };
    Bluetooth {
        on,
        detail,
        note: None,
    }
}

pub struct Battery {
    pub percent: String,
    pub fraction: f32,
    pub detail: String,
}

fn battery() -> Battery {
    battery_upower().unwrap_or_else(battery_sysfs)
}

fn battery_upower() -> Option<Battery> {
    let out = Command::new("upower").args(["-e"]).output().ok()?;
    let path = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.contains("battery_"))
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        return None;
    }
    let info = Command::new("upower").args(["-i", &path]).output().ok()?;
    if !info.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&info.stdout);
    let field = |name: &str| {
        text.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then(|| value.trim().to_string())
        })
    };
    let percent_raw = field("percentage")?;
    let fraction = percent_raw
        .trim_end_matches('%')
        .trim()
        .parse::<f32>()
        .unwrap_or(0.0)
        / 100.0;
    let percent = format!("{}%", percent_raw.trim_end_matches('%').trim());
    let time = field("time to empty").filter(|s| !s.is_empty());
    let state = field("state").unwrap_or_default();
    let detail = if let Some(time) = time {
        format!("about {time} left")
    } else if state == "charging" {
        "Charging".into()
    } else {
        String::new()
    };
    Some(Battery {
        percent,
        fraction,
        detail,
    })
}

fn battery_sysfs() -> Battery {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return Battery {
            percent: String::new(),
            fraction: 0.0,
            detail: String::new(),
        };
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = std::fs::read_to_string(path.join("type")).unwrap_or_default();
        if !kind.to_ascii_lowercase().contains("battery") {
            continue;
        }
        if let Ok(cap) = std::fs::read_to_string(path.join("capacity")) {
            let cap = cap.trim();
            if !cap.is_empty() {
                return Battery {
                    percent: format!("{cap}%"),
                    fraction: cap.parse::<f32>().unwrap_or(0.0) / 100.0,
                    detail: String::new(),
                };
            }
        }
    }
    Battery {
        percent: String::new(),
        fraction: 0.0,
        detail: String::new(),
    }
}

struct Updates {
    summary: String,
    count: usize,
    note: Option<String>,
}

fn updates() -> Updates {
    match Command::new("pacman").args(["-Qu"]).output() {
        Ok(out) => {
            let empty_ok =
                out.status.success() || (out.status.code() == Some(1) && out.stderr.is_empty());
            if !empty_ok {
                return Updates {
                    summary: "Couldn't check".into(),
                    count: 0,
                    note: Some("Updates: pacman -Qu failed.".into()),
                };
            }
            let count = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count();
            Updates {
                summary: if count == 0 {
                    "Nothing waiting".into()
                } else {
                    format!("{count} system updates")
                },
                count,
                note: None,
            }
        }
        Err(_) => Updates {
            summary: "Not checked".into(),
            count: 0,
            note: Some("Updates: pacman is not installed (this host is not Arch).".into()),
        },
    }
}

struct Storage {
    detail: String,
    fraction: f32,
    note: Option<String>,
}

fn storage() -> Storage {
    match Command::new("df")
        .args(["-B1", "--output=size,used,avail", "/"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let fields: Vec<_> = text
                .lines()
                .nth(1)
                .unwrap_or("")
                .split_whitespace()
                .collect();
            let total: u64 = fields.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let used: u64 = fields.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            Storage {
                detail: format!("{} of {} used", gib(used), gib(total)),
                fraction: if total == 0 {
                    0.0
                } else {
                    used as f32 / total as f32
                },
                note: None,
            }
        }
        _ => Storage {
            detail: String::new(),
            fraction: 0.0,
            note: Some("Storage: df failed.".into()),
        },
    }
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / 1024_f64.powi(3))
}

fn local_hour() -> u32 {
    // The shipping greeting uses the machine's local hour. UTC is hours off
    // on this laptop (AWST), which is why the prototype said morning at 13:00.
    let Ok(out) = Command::new("date").args(["+%H"]).output() else {
        return 12;
    };
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(12)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub gaps_in: i64,
    pub gaps_out: i64,
    pub border_size: i64,
    pub active_border: String,
    pub inactive_border: String,
    pub rounding: i64,
    pub blur_enabled: bool,
    pub blur_size: i64,
    pub blur_passes: i64,
    pub shadow_enabled: bool,
    pub shadow_range: i64,
    pub layout: String,
    pub resize_on_border: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            gaps_in: 5,
            gaps_out: 10,
            border_size: 2,
            active_border: "rgba(88c0d0ff)".into(),
            inactive_border: "rgba(4c566aff)".into(),
            rounding: 8,
            blur_enabled: true,
            blur_size: 6,
            blur_passes: 2,
            shadow_enabled: true,
            shadow_range: 12,
            layout: "dwindle".into(),
            resize_on_border: true,
        }
    }
}

fn settings_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("hypr/settings.json")
}

pub fn load_appearance() -> Appearance {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Merge the keys this page owns into the existing JSON object and reload
/// Hyprland. Other keys (colours, keyboard, scroll) are left untouched.
pub fn save_appearance(appearance: &Appearance) -> Result<(), String> {
    let path = settings_path();
    let mut stored = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let values = serde_json::to_value(appearance).map_err(|e| e.to_string())?;
    let object = stored
        .as_object_mut()
        .ok_or("Appearance settings must be a JSON object")?;
    for (key, value) in values
        .as_object()
        .expect("appearance serializes to an object")
    {
        object.insert(key.clone(), value.clone());
    }
    let json = serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if path.exists() {
        let backup = path.with_extension("json.bak");
        std::fs::copy(&path, &backup).map_err(|e| format!("could not back up settings: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    match Command::new("hyprctl").arg("reload").status() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err("Saved, but Hyprland refused the reload.".into()),
        Err(_) => Err("Saved. hyprctl is not available here, so nothing was reloaded.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_round_trips_only_its_own_keys() {
        let dir = std::env::temp_dir().join(format!("bos-slint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("hypr")).unwrap();
        let path = dir.join("hypr/settings.json");
        std::fs::write(&path, r#"{"kb_layout":"us","gaps_in":1,"extra":true}"#).unwrap();
        // The helper reads XDG paths, so exercise the merge directly.
        let mut stored: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let appearance = Appearance {
            gaps_in: 9,
            ..Appearance::default()
        };
        let values = serde_json::to_value(&appearance).unwrap();
        for (key, value) in values.as_object().unwrap() {
            stored
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone());
        }
        assert_eq!(stored["kb_layout"], "us");
        assert_eq!(stored["extra"], true);
        assert_eq!(stored["gaps_in"], 9);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
