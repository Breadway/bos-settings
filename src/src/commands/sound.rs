//! Output/input volume and device selection over PipeWire's pulse
//! compatibility layer (`pactl`), the same surface hyprland.lua's media
//! keys already use via `wpctl`. `pactl` is used here instead because it
//! can enumerate devices with human-readable descriptions and switch the
//! default in one command; `wpctl` cannot easily do either.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

static WATCHING: AtomicBool = AtomicBool::new(false);

/// `pactl subscribe` prints one line per server event, e.g.
/// `Event 'change' on sink #53`, `Event 'change' on server`,
/// `Event 'new' on sink-input #12`. Volume, mute and default-device changes
/// arrive as `change` on sink / source / server; per-app streams are
/// `sink-input`. Card, client and module chatter is ignored.
fn is_relevant_event(line: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix("Event '") else {
        return false;
    };
    let Some((kind, target)) = rest.split_once("' on ") else {
        return false;
    };
    if !matches!(kind, "change" | "new" | "remove") {
        return false;
    }
    let object = target.split_whitespace().next().unwrap_or("");
    matches!(object, "sink" | "source" | "server" | "sink-input")
}

/// Start (once per process) a `pactl subscribe` watcher that emits
/// `sound-changed` whenever the output/input state may have changed, so the
/// Sound page follows volume keys, the bar and other mixers (BSET-31).
/// Bursts (a volume drag fires dozens of events) are coalesced into one emit.
#[tauri::command]
pub fn watch_sound(app: AppHandle) {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            subscribe_once(&app).await;
            // PipeWire restarted or pactl missing: back off and retry.
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
}

async fn subscribe_once(app: &AppHandle) {
    let Ok(mut child) = Command::new("pactl")
        .arg("subscribe")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
    else {
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        return;
    };
    let mut lines = BufReader::new(stdout).lines();
    let mut dirty = false;
    loop {
        let next = if dirty {
            tokio::time::timeout(Duration::from_millis(150), lines.next_line()).await
        } else {
            Ok(lines.next_line().await)
        };
        match next {
            Ok(Ok(Some(line))) => {
                if is_relevant_event(&line) {
                    dirty = true;
                }
            }
            Ok(_) => break,
            Err(_) => {
                let _ = app.emit("sound-changed", ());
                dirty = false;
            }
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SoundDevice {
    name: String,
    description: String,
    mute: bool,
    percent: f64,
    /// Monitor sources and loopback devices: real in PipeWire terms, but not
    /// something a person picks as a speaker or microphone.
    virtual_device: bool,
}

fn is_virtual_device(name: &str, description: &str) -> bool {
    name.ends_with(".monitor")
        || description.starts_with("Monitor of ")
        || name.to_ascii_lowercase().contains("loopback")
        || description.to_ascii_lowercase().contains("loopback")
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
            .and_then(|v| {
                v.value_percent
                    .trim_end_matches('%')
                    .trim()
                    .parse::<f64>()
                    .ok()
            })
            .unwrap_or(0.0)
    }
}

async fn list_devices(kind: &str) -> Vec<SoundDevice> {
    let Ok(output) = Command::new("pactl")
        .args(["-f", "json", "list", kind])
        .output()
        .await
    else {
        return Vec::new();
    };
    let raw: Vec<RawDevice> = serde_json::from_slice(&output.stdout).unwrap_or_default();
    raw.into_iter()
        .map(|d| SoundDevice {
            virtual_device: is_virtual_device(&d.name, &d.description),
            name: d.name.clone(),
            description: d.description.clone(),
            mute: d.mute,
            percent: d.percent(),
        })
        .collect()
}

async fn default_device_name(kind: &str) -> Option<String> {
    let flag = if kind == "sinks" {
        "get-default-sink"
    } else {
        "get-default-source"
    };
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
    SoundSection {
        devices,
        default_name,
    }
}

#[tauri::command]
pub async fn set_default_sound_device(kind: String, name: String) -> Result<(), String> {
    let flag = if kind == "sinks" {
        "set-default-sink"
    } else {
        "set-default-source"
    };
    Command::new("pactl")
        .args([flag, &name])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_sound_volume(kind: String, name: String, percent: i64) -> Result<(), String> {
    let flag = if kind == "sinks" {
        "set-sink-volume"
    } else {
        "set-source-volume"
    };
    let pct = format!("{percent}%");
    Command::new("pactl")
        .args([flag, &name, &pct])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_sound_mute(kind: String, name: String, mute: bool) -> Result<(), String> {
    let flag = if kind == "sinks" {
        "set-sink-mute"
    } else {
        "set-source-mute"
    };
    let val = if mute { "1" } else { "0" };
    Command::new("pactl")
        .args([flag, &name, val])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct AppStream {
    index: u32,
    app: String,
    title: String,
    mute: bool,
    percent: f64,
}

#[derive(Deserialize)]
struct RawStream {
    index: u32,
    mute: bool,
    volume: HashMap<String, RawVolumeChannel>,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

fn prop(props: &HashMap<String, serde_json::Value>, key: &str) -> String {
    props
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn parse_streams(json: &[u8]) -> Vec<AppStream> {
    let raw: Vec<RawStream> = serde_json::from_slice(json).unwrap_or_default();
    raw.into_iter()
        .map(|s| {
            let percent = s
                .volume
                .values()
                .next()
                .and_then(|v| v.value_percent.trim_end_matches('%').trim().parse().ok())
                .unwrap_or(0.0);
            let app = prop(&s.properties, "application.name");
            AppStream {
                index: s.index,
                app: if app.is_empty() {
                    "Unknown app".into()
                } else {
                    app
                },
                title: prop(&s.properties, "media.name"),
                mute: s.mute,
                percent,
            }
        })
        .collect()
}

/// Streams currently playing, for per-app volume on the Sound page.
#[tauri::command]
pub async fn get_app_streams() -> Vec<AppStream> {
    match Command::new("pactl")
        .args(["-f", "json", "list", "sink-inputs"])
        .output()
        .await
    {
        Ok(out) => parse_streams(&out.stdout),
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub async fn set_app_stream_volume(index: u32, percent: u32) -> Result<(), String> {
    let pct = format!("{}%", percent.min(150));
    let status = Command::new("pactl")
        .args(["set-sink-input-volume", &index.to_string(), &pct])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("That app stopped playing".into())
    }
}

#[tauri::command]
pub async fn set_app_stream_mute(index: u32, mute: bool) -> Result<(), String> {
    let status = Command::new("pactl")
        .args([
            "set-sink-input-mute",
            &index.to_string(),
            if mute { "1" } else { "0" },
        ])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("That app stopped playing".into())
    }
}

#[tauri::command]
pub fn open_mixer() {
    let _ = std::process::Command::new("pavucontrol").spawn();
}

#[cfg(test)]
mod tests {
    use super::{is_relevant_event, is_virtual_device, parse_streams};

    #[test]
    fn monitor_and_loopback_devices_are_virtual() {
        assert!(is_virtual_device(
            "alsa_output.pci.analog-stereo.monitor",
            "Monitor of Speaker"
        ));
        assert!(is_virtual_device(
            "input.loopback",
            "Loopback Analog Stereo"
        ));
        assert!(!is_virtual_device(
            "alsa_input.pci.digital-mic",
            "Digital Microphone"
        ));
    }

    #[test]
    fn streams_parse_from_pactl_json() {
        let json = br#"[{"index":7,"mute":false,"volume":{"front-left":{"value_percent":"95%"}},"properties":{"application.name":"Zen","media.name":"Lucent"}},{"index":8,"mute":true,"volume":{},"properties":{}}]"#;
        let s = parse_streams(json);
        assert_eq!(s.len(), 2);
        assert_eq!(
            (
                s[0].index,
                s[0].app.as_str(),
                s[0].title.as_str(),
                s[0].percent
            ),
            (7, "Zen", "Lucent", 95.0)
        );
        assert_eq!(s[1].app, "Unknown app");
        assert!(s[1].mute);
    }

    #[test]
    fn volume_mute_and_default_changes_are_relevant() {
        assert!(is_relevant_event("Event 'change' on sink #53"));
        assert!(is_relevant_event("Event 'change' on source #54"));
        assert!(is_relevant_event("Event 'change' on server"));
        assert!(is_relevant_event("Event 'new' on sink-input #12"));
        assert!(is_relevant_event("Event 'remove' on sink-input #12\n"));
    }

    #[test]
    fn card_client_and_module_chatter_is_not() {
        assert!(!is_relevant_event("Event 'change' on card #1"));
        assert!(!is_relevant_event("Event 'new' on client #99"));
        assert!(!is_relevant_event("Event 'new' on module #3"));
        assert!(!is_relevant_event("Event 'change' on source-output #7"));
    }

    #[test]
    fn non_event_lines_are_ignored() {
        assert!(!is_relevant_event(""));
        assert!(!is_relevant_event("Connection failure: Connection refused"));
        assert!(!is_relevant_event("Event 'bogus' on sink #1"));
        assert!(!is_relevant_event("Event on sink #1"));
    }
}
