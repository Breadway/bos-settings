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
    pub night_installed: bool,
    pub night_on: bool,
    pub dnd_on: bool,
    pub dnd_detail: String,
    pub power_mode: String,
    pub power_mode_detail: String,
    pub automatic_power: bool,
    pub airplane_available: bool,
    pub airplane_on: bool,
    pub connected: String,
    pub backup_ready: bool,
}

pub fn load_home() -> Home {
    let account = account();
    let wifi = wifi();
    let bluetooth = bluetooth();
    let battery = battery();
    let updates = updates();
    let storage = storage();
    let night = crate::nightlight::load();
    let night_installed = command_exists("hyprsunset");
    let (dnd_on, dnd_detail) = notification_state();
    let (power_mode, power_mode_detail, automatic_power) = power_state();
    let (airplane_available, airplane_on) = airplane_state();
    let backup_ready = backup_repo().is_some();
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
        night_installed,
        night_on: night.enabled,
        dnd_on,
        dnd_detail,
        power_mode,
        power_mode_detail,
        automatic_power,
        airplane_available,
        airplane_on,
        connected: bluetooth.connected.join(", "),
        backup_ready,
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
    connected: Vec<String>,
}

fn bluetooth() -> Bluetooth {
    let Ok(out) = Command::new("bluetoothctl").args(["show"]).output() else {
        return Bluetooth {
            on: false,
            detail: "Unavailable".into(),
            note: Some("Bluetooth: bluetoothctl is not installed.".into()),
            connected: Vec::new(),
        };
    };
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() || !text.trim_start().starts_with("Controller") {
        return Bluetooth {
            on: false,
            detail: "No adapter".into(),
            note: None,
            connected: Vec::new(),
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
        connected,
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

pub fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null 2>&1", "sh", name])
        .status()
        .is_ok_and(|status| status.success())
}

fn notification_state() -> (bool, String) {
    let Ok(prefs) = crate::notifications::load() else {
        return (false, "Not available".into());
    };
    let effective = prefs
        .get("effective_dnd")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| prefs.get("dnd").and_then(serde_json::Value::as_bool))
        .unwrap_or(false);
    let scheduled = prefs
        .get("schedule")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    (
        effective,
        if effective {
            "On".into()
        } else if scheduled {
            "Scheduled".into()
        } else {
            "Off".into()
        },
    )
}

fn power_state() -> (String, String, bool) {
    let read = |program: &str| {
        Command::new(program)
            .arg("get")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let current = read("powerprofilesctl")
        .or_else(|| read("tlpctl"))
        .unwrap_or_default();
    let detail = match current.as_str() {
        "power-saver" => "Battery saver",
        "balanced" => "Balanced",
        "performance" => "Performance",
        _ => "Not available",
    };
    let automatic = Command::new("systemctl")
        .args(["is-enabled", "bos-settings-power-auto.timer"])
        .output()
        .is_ok_and(|out| out.status.success());
    (current, detail.into(), automatic)
}

fn airplane_state() -> (bool, bool) {
    let Ok(out) = Command::new("rfkill").args(["--json"]).output() else {
        return (false, false);
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return (false, false);
    };
    let Some(rows) = value.get("rfkilldevices").and_then(|v| v.as_array()) else {
        return (false, false);
    };
    let radios: Vec<_> = rows
        .iter()
        .filter(|row| {
            matches!(
                row.get("type").and_then(|v| v.as_str()),
                Some("wlan" | "bluetooth" | "wwan")
            )
        })
        .collect();
    if radios.is_empty() {
        return (false, false);
    }
    (
        true,
        radios.iter().all(|row| {
            row.get("soft")
                .and_then(|v| v.as_str())
                .is_some_and(|state| state == "blocked")
                || row
                    .get("hard")
                    .and_then(|v| v.as_str())
                    .is_some_and(|state| state == "blocked")
        }),
    )
}

pub fn backup_repo() -> Option<String> {
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("bos-settings/backup.toml");
    let text = std::fs::read_to_string(path).ok()?;
    let doc = text.parse::<toml_edit::DocumentMut>().ok()?;
    doc.get("repo")
        .and_then(toml_edit::Item::as_str)
        .map(str::trim)
        .filter(|repo| !repo.is_empty())
        .map(str::to_string)
}

pub fn set_wifi_radio(enabled: bool) -> Result<(), String> {
    let out = Command::new("nmcli")
        .args(["radio", "wifi", if enabled { "on" } else { "off" }])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn set_bluetooth_power(on: bool) -> Result<(), String> {
    let out = Command::new("bluetoothctl")
        .args(["power", if on { "on" } else { "off" }])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn set_power_mode(profile: &str) -> Result<(), String> {
    let program = if command_exists("powerprofilesctl") {
        "powerprofilesctl"
    } else {
        "tlpctl"
    };
    let out = Command::new(program)
        .args(["set", profile])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn set_airplane(enabled: bool) -> Result<(), String> {
    let action = if enabled { "block" } else { "unblock" };
    let ids = Command::new("rfkill")
        .args(["--json"])
        .output()
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value =
        serde_json::from_slice(&ids.stdout).map_err(|error| error.to_string())?;
    let rows = value
        .get("rfkilldevices")
        .and_then(|v| v.as_array())
        .ok_or("invalid radio status")?;
    let mut ids = Vec::new();
    for row in rows {
        if matches!(
            row.get("type").and_then(|v| v.as_str()),
            Some("wlan" | "bluetooth" | "wwan")
        ) {
            if let Some(id) = row.get("id").and_then(|v| v.as_u64()) {
                ids.push(id.to_string());
            }
        }
    }
    if ids.is_empty() {
        return Err("No wireless radios found".into());
    }
    let out = Command::new("rfkill")
        .arg(action)
        .args(&ids)
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
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

// ---- Appearance hub: shell themes, wallpapers, text settings ----

pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    /// Base theme whose look the preview should show: the first id in the
    /// `extends` chain that names a real theme.
    pub preview: String,
    pub custom: bool,
}

pub struct LibItem {
    pub name: String,
    pub path: String,
}

pub struct Extra {
    pub themes: Vec<ThemeInfo>,
    pub active_theme: String,
    pub wallpaper: Option<String>,
    pub wallpaper_name: String,
    pub library_dir: String,
    pub library: Vec<LibItem>,
    pub text_size: i64,
    pub font: String,
}

/// bread_theme's compiled-in themes, in presentation order.
const BUILTIN_THEMES: &[(&str, &str)] = &[
    ("liquid-motion", "Liquid Motion"),
    ("glass-workbench", "Glass Workbench"),
    ("spotlight", "Spotlight"),
    ("daylight", "Daylight"),
    ("loaf", "Loaf"),
];

const WALLPAPER_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];
const MAX_THUMBS: usize = 12;

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root"))
}

fn shell_toml() -> PathBuf {
    config_dir().join("bread/shell.toml")
}

pub fn load_extra() -> Extra {
    let (themes, active_theme) = themes();
    let (wallpaper, wallpaper_name) = current_wallpaper();
    let library_dir = home_dir().join("Pictures/Backgrounds");
    Extra {
        themes,
        active_theme,
        wallpaper,
        wallpaper_name,
        library_dir: library_dir.to_string_lossy().to_string(),
        library: library(&library_dir),
        text_size: text_prefs().0,
        font: text_prefs().1,
    }
}

fn themes() -> (Vec<ThemeInfo>, String) {
    let mut out: Vec<ThemeInfo> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for dir in [
        config_dir().join("bread/themes"),
        PathBuf::from("/usr/share/bread/themes"),
    ] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let id = entry.file_name().to_string_lossy().to_string();
            if seen.contains(&id) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(entry.path().join("theme.toml")) else {
                continue;
            };
            let Ok(value) = text.parse::<toml::Value>() else {
                continue;
            };
            let name = value
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(&id)
                .to_string();
            let preview = base_theme(&value, &id, 0);
            seen.push(id.clone());
            out.push(ThemeInfo {
                id,
                name,
                preview,
                custom: true,
            });
        }
    }
    for (id, name) in BUILTIN_THEMES {
        if !seen.iter().any(|s| s == id) {
            out.push(ThemeInfo {
                id: (*id).into(),
                name: (*name).into(),
                preview: (*id).into(),
                custom: false,
            });
        }
    }
    (out, active_theme())
}

/// A theme that only overrides tokens (spotlight-varela) should preview as
/// the theme it extends, not as the generic bar.
fn base_theme(value: &toml::Value, fallback: &str, depth: usize) -> String {
    if depth > 8 {
        return fallback.into();
    }
    let Some(base) = value.get("extends").and_then(|e| e.as_str()) else {
        return fallback.into();
    };
    // Follow the chain through this machine's theme dirs if the parent is a
    // user theme too.
    for dir in [
        config_dir().join("bread/themes"),
        PathBuf::from("/usr/share/bread/themes"),
    ] {
        if let Ok(text) = std::fs::read_to_string(dir.join(base).join("theme.toml")) {
            if let Ok(parent) = text.parse::<toml::Value>() {
                return base_theme(&parent, base, depth + 1);
            }
        }
    }
    base.into()
}

fn active_theme() -> String {
    std::fs::read_to_string(shell_toml())
        .ok()
        .and_then(|s| s.parse::<toml::Value>().ok())
        .and_then(|v| v.get("active").and_then(|a| a.as_str()).map(str::to_string))
        .unwrap_or_else(|| "liquid-motion".into())
}

pub fn set_theme(id: &str) -> Result<(), String> {
    let path = shell_toml();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let line = format!("active = \"{id}\"");
    let mut out: Vec<String> = Vec::new();
    let mut replaced = false;
    for existing in text.lines() {
        if !replaced && existing.trim_start().starts_with("active") && existing.contains('=') {
            out.push(line.clone());
            replaced = true;
        } else {
            out.push(existing.to_string());
        }
    }
    if !replaced {
        out.push(line);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, out.join("\n") + "\n").map_err(|e| e.to_string())
}

pub fn restart_shell() -> Result<(), String> {
    use std::process::{Command, Stdio};
    for app in ["breadbar", "breadbox"] {
        let _ = Command::new("pkill").args(["-x", app]).status();
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    let spawn = |prog: &str, args: &[&str]| {
        let _ = Command::new("setsid")
            .arg("-f")
            .arg(prog)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    };
    spawn("breadbar", &[]);
    spawn("breadbox", &["listen"]);
    Ok(())
}

fn current_wallpaper() -> (Option<String>, String) {
    let out = Command::new("breadpaper").arg("get").output();
    let path = out
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    let name = path
        .as_deref()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("")
        .to_string();
    (path, name)
}

fn library(dir: &PathBuf) -> Vec<LibItem> {
    fn walk(dir: &std::path::Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth == 0 || out.len() >= MAX_THUMBS {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if out.len() >= MAX_THUMBS {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth - 1, out);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| WALLPAPER_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    walk(dir, 2, &mut paths);
    paths
        .into_iter()
        .map(|p| LibItem {
            name: p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: p.to_string_lossy().to_string(),
        })
        .collect()
}

pub fn set_wallpaper(path: &str) -> Result<(), String> {
    let status = Command::new("breadpaper")
        .args(["set", path])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("breadpaper refused the wallpaper".into())
    }
}

pub fn choose_wallpaper() -> Option<String> {
    let out = Command::new("zenity")
        .args([
            "--file-selection",
            "--title=Choose a wallpaper",
            "--file-filter=Images | *.png *.jpg *.jpeg *.webp *.gif *.bmp",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

fn text_prefs() -> (i64, String) {
    let path = config_dir().join("hypr/bos-settings-v2.json");
    let value: serde_json::Value = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    (
        value
            .get("text_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(14) as i64,
        value
            .get("font_family")
            .and_then(|v| v.as_str())
            .unwrap_or("Varela Round")
            .to_string(),
    )
}

pub fn save_text(size: i64, font: &str) -> Result<(), String> {
    let path = config_dir().join("hypr/bos-settings-v2.json");
    let mut value: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    value["text_size"] = serde_json::json!(size);
    value["font_family"] = serde_json::json!(font);
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    // GTK apps read gtk-font-name from their settings.ini.
    for folder in ["gtk-3.0", "gtk-4.0"] {
        let ini = config_dir().join(folder).join("settings.ini");
        let text = std::fs::read_to_string(&ini).unwrap_or_default();
        let text = set_ini_key(&text, "gtk-font-name", &format!("{font} {size}"));
        let _ = std::fs::create_dir_all(ini.parent().unwrap_or(&config_dir()));
        let _ = std::fs::write(&ini, text);
    }
    Ok(())
}

fn set_ini_key(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let start = lines.iter().position(|l| l.trim() == "[Settings]");
    let Some(start) = start else {
        lines.push("[Settings]".into());
        lines.push(format!("{key}={value}"));
        return lines.join("\n") + "\n";
    };
    let end = (start + 1..lines.len())
        .find(|&i| lines[i].trim().starts_with('['))
        .unwrap_or(lines.len());
    let found = (start + 1..end).find(|&i| {
        lines[i]
            .split_once('=')
            .is_some_and(|(k, _)| k.trim() == key)
    });
    match found {
        Some(i) => lines[i] = format!("{key}={value}"),
        None => lines.insert(end, format!("{key}={value}")),
    }
    lines.join("\n") + "\n"
}

// ---- About ----

pub struct About {
    pub os: String,
    pub version: Option<String>,
    pub kernel: String,
    pub cpu: String,
    pub gpu: String,
    pub gpu_full: String,
    pub memory: String,
    pub disk: String,
    pub uptime: String,
    pub hostname: String,
    pub channel: String,
}

pub struct HealthCheck {
    pub name: String,
    pub ok: bool,
    pub output: String,
}

pub fn load_about() -> About {
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |key: &str| {
        os_release.lines().find_map(|line| {
            line.strip_prefix(key)
                .map(|rest| rest.trim_start_matches('=').trim_matches('"').to_string())
        })
    };
    let os = field("PRETTY_NAME").unwrap_or_else(|| "BOS".into());
    let version = field("VERSION_ID").filter(|v| !v.is_empty());
    let kernel = command_line("uname", &["-r"]).unwrap_or_else(|| "unknown".into());
    let gpu_full = gpu_name();
    About {
        os,
        version,
        kernel,
        cpu: cpu_name(),
        gpu: friendly_gpu(&gpu_full),
        gpu_full,
        memory: memory_total(),
        disk: disk_usage(),
        uptime: command_line("uptime", &["-p"]).unwrap_or_else(|| "unknown".into()),
        hostname: std::fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".into()),
        channel: bakery_track(),
    }
}

fn command_line(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn cpu_name() -> String {
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                line.strip_prefix("model name")
                    .map(|rest| rest.trim_start_matches([':', ' ', '\t']).to_string())
            })
        })
        .unwrap_or_else(|| "unknown".into());
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    if threads > 0 {
        format!("{model} ({threads} threads)")
    } else {
        model
    }
}

fn gpu_name() -> String {
    let Some(text) = command_line("lspci", &[]) else {
        return "unknown".into();
    };
    text.lines()
        .find(|line| {
            line.contains("VGA compatible controller")
                || line.contains("3D controller")
                || line.contains("Display controller")
        })
        .and_then(|line| line.split(": ").nth(1))
        .unwrap_or("unknown")
        .to_string()
}

/// Port of About.svelte's friendlyGpu: keep the marketing model out of a
/// raw lspci string ("... [Radeon 840M / 860M Graphics] (rev c2)").
fn friendly_gpu(raw: &str) -> String {
    if raw.is_empty() {
        return raw.into();
    }
    let mut text = raw.trim().to_string();
    if let Some(start) = text.rfind(" (rev ") {
        if text.ends_with(')') {
            text.truncate(start);
        }
    }
    if let Some(open) = text.rfind('[') {
        if let Some(close) = text[open..].find(']') {
            text = text[open + 1..open + close].to_string();
        }
    } else if let Some(pos) = text.find(", Inc. ") {
        text = text[pos + 6..].to_string();
    }
    if let Some(slash) = text.find(" / ") {
        let head = text.split_whitespace().next().unwrap_or("").to_string();
        let tail = text[slash + 3..].split_whitespace().next().unwrap_or("");
        if !head.is_empty() && !tail.is_empty() {
            text = format!("{head} {tail}");
        }
    }
    for suffix in [" Graphics", " Controller", " Series"] {
        if text.ends_with(suffix) {
            let len = text.len() - suffix.len();
            text.truncate(len);
        }
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        raw.into()
    } else {
        text
    }
}

fn memory_total() -> String {
    let kb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("MemTotal:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        });
    match kb {
        Some(kb) => format!("{:.1} GiB", kb as f64 / 1024.0 / 1024.0),
        None => "unknown".into(),
    }
}

fn disk_usage() -> String {
    let Some(text) = command_line("df", &["-h", "--output=used,size,pcent", "/"]) else {
        return "unknown".into();
    };
    text.lines()
        .nth(1)
        .map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            match cols.as_slice() {
                [used, size, pcent] => format!("{used} of {size} used ({pcent})"),
                _ => line.trim().to_string(),
            }
        })
        .unwrap_or_else(|| "unknown".into())
}

fn bakery_track() -> String {
    command_line("bakery", &["track", "show"])
        .and_then(|text| {
            text.split_whitespace()
                .last()
                .map(|track| track.to_string())
        })
        .unwrap_or_else(|| "Unavailable".into())
}

pub fn health_checks() -> Vec<HealthCheck> {
    let mut out = Vec::new();
    for (name, program, arg) in [
        ("Bread daemon and desktop", "bread", "doctor"),
        ("Packages", "bakery", "doctor"),
    ] {
        let (ok, output) = match Command::new(program).arg(arg).output() {
            Ok(result) => (
                result.status.success(),
                format!(
                    "{}{}",
                    String::from_utf8_lossy(&result.stdout),
                    String::from_utf8_lossy(&result.stderr)
                ),
            ),
            Err(err) => (false, err.to_string()),
        };
        out.push(HealthCheck {
            name: name.into(),
            ok,
            output: output.trim().to_string(),
        });
    }
    out
}

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("no clipboard tool: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}
