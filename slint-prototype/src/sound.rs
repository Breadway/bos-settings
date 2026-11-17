//! PipeWire sound control through pactl.

use std::process::Command;

#[derive(Clone, Debug)]
pub struct Device {
    pub name: String,
    pub description: String,
    pub mute: bool,
    pub percent: u32,
    pub virtual_device: bool,
}

#[derive(Clone, Debug)]
pub struct AppStream {
    pub index: u32,
    pub app: String,
    pub title: String,
    pub mute: bool,
    pub percent: u32,
}

fn text(args: &[&str]) -> Option<String> {
    let out = Command::new("pactl").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn percent(value: &serde_json::Value) -> u32 {
    let volume = value.get("volume").and_then(|v| v.as_object());
    let first = volume.and_then(|volume| volume.values().next());
    let raw = first
        .and_then(|entry| entry.get("value"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    (raw / 65536.0 * 100.0).round() as u32
}

fn mute(value: &serde_json::Value) -> bool {
    value.get("mute").and_then(|v| v.as_bool()).unwrap_or(false)
}

pub fn devices(kind: &str) -> Vec<Device> {
    let command = if kind == "sinks" { "list" } else { "list" };
    let Some(raw) = text(&["-f", "json", command, kind]) else {
        return Vec::new();
    };
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    rows.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let name = row.get("name")?.as_str()?.to_string();
                    let description = row
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&name)
                        .to_string();
                    let lower = format!("{} {}", name.to_ascii_lowercase(), description.to_ascii_lowercase());
                    Some(Device {
                        virtual_device: lower.contains("monitor")
                            || lower.contains("loopback")
                            || lower.contains("null"),
                        name,
                        description,
                        mute: mute(row),
                        percent: percent(row),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn default_device(kind: &str) -> Option<String> {
    text(&[
        "get-default-sink"
            .replace("sink", if kind == "sinks" { "sink" } else { "source" })
            .as_str(),
    ])
}

pub fn set_default(kind: &str, name: &str) -> Result<(), String> {
    let verb = if kind == "sinks" {
        "set-default-sink"
    } else {
        "set-default-source"
    };
    run(&[verb, name])
}

pub fn set_volume(kind: &str, name: &str, value: u32) -> Result<(), String> {
    let verb = format!(
        "set-{}-volume",
        if kind == "sinks" { "sink" } else { "source" }
    );
    run(&[&verb, name, &format!("{}%", value.min(150))])
}

pub fn set_mute(kind: &str, name: &str, value: bool) -> Result<(), String> {
    let verb = format!(
        "set-{}-mute",
        if kind == "sinks" { "sink" } else { "source" }
    );
    run(&[&verb, name, if value { "1" } else { "0" }])
}

pub fn streams() -> Vec<AppStream> {
    let Some(raw) = text(&["-f", "json", "list", "sink-inputs"]) else {
        return Vec::new();
    };
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    rows.as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let index = row.get("index")?.as_u64()? as u32;
                    let props = row.get("properties").and_then(|v| v.as_object());
                    let app = props
                        .and_then(|props| props.get("application.name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Audio")
                        .to_string();
                    let title = props
                        .and_then(|props| props.get("media.name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    Some(AppStream {
                        index,
                        app,
                        title,
                        mute: mute(row),
                        percent: percent(row),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn set_stream_volume(index: u32, value: u32) -> Result<(), String> {
    run(&[
        "set-sink-input-volume",
        &index.to_string(),
        &format!("{}%", value.min(150)),
    ])
}

pub fn set_stream_mute(index: u32, value: bool) -> Result<(), String> {
    run(&[
        "set-sink-input-mute",
        &index.to_string(),
        if value { "1" } else { "0" },
    ])
}

pub fn open_mixer() -> Result<(), String> {
    Command::new("pavucontrol")
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn run(args: &[&str]) -> Result<(), String> {
    let out = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_percent_is_scaled_from_pulse_units() {
        let value = serde_json::json!({
            "volume": { "front-left": { "value": 65536 }, "front-right": { "value": 65536 } },
            "mute": true
        });
        assert_eq!(percent(&value), 100);
        assert!(mute(&value));
    }

    #[test]
    fn within_range_is_capped() {
        assert_eq!(format!("{}%", 200.min(150)), "150%");
    }
}
