//! Local bug report settings and history. Reports are read from the files
//! breadbug writes; nothing here sends anything.

use std::path::PathBuf;
use std::process::Command;

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

pub fn config_path() -> PathBuf {
    config_dir().join("bread").join("breadbug.toml")
}

pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"))
        .join("bread")
        .join("breadbug")
}

pub struct Settings {
    pub installed: bool,
    pub upload: bool,
    pub notify: bool,
    pub excluded: Vec<String>,
}

pub fn load() -> Settings {
    let text = std::fs::read_to_string(config_path()).unwrap_or_default();
    let value = text.parse::<toml_edit::DocumentMut>().ok();
    let flag = |key: &str| {
        value
            .as_ref()
            .and_then(|doc| doc.get(key))
            .and_then(|item| item.as_bool())
            .unwrap_or(false)
    };
    let excluded = value
        .as_ref()
        .and_then(|doc| doc.get("excluded_apps"))
        .and_then(|item| item.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Settings {
        installed: which("breadbug"),
        upload: flag("upload_enabled"),
        notify: flag("notify_fixed"),
        excluded,
    }
}

/// Writes only the keys this page owns and leaves comments and unknown
/// keys untouched.
pub fn save(upload: bool, notify: bool) -> Result<(), String> {
    let path = config_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|error| format!("{error}"))?;
    doc["upload_enabled"] = toml_edit::value(upload);
    doc["notify_fixed"] = toml_edit::value(notify);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, doc.to_string()).map_err(|error| error.to_string())
}

pub struct Report {
    pub when: u64,
    pub kind: String,
    pub app: String,
    pub status: String,
    pub path: Option<String>,
}

pub fn history() -> Vec<Report> {
    let text = std::fs::read_to_string(state_dir().join("history.jsonl")).unwrap_or_default();
    let mut rows: Vec<Report> = text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|value| Report {
            when: value.get("t").and_then(|v| v.as_u64()).unwrap_or(0),
            kind: value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("saved")
                .into(),
            app: value
                .get("app")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .into(),
            status: value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .into(),
            path: value
                .get("path")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
        .collect();
    rows.reverse();
    rows
}

pub fn queued() -> usize {
    std::fs::read_dir(state_dir().join("queue"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "meta"))
                .count()
        })
        .unwrap_or(0)
}

pub fn clear_queue() -> usize {
    let dir = state_dir().join("queue");
    let mut removed = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if std::fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

pub fn wipe_local() -> Result<(), String> {
    let apps = state_dir().join("bugstore/apps");
    match std::fs::remove_dir_all(&apps) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not delete {}: {error}", apps.display())),
    }
}

pub fn open_reporter() -> Result<(), String> {
    if !which("breadbug") {
        return Err("breadbug is not installed".into());
    }
    Command::new("kitty")
        .args(["--class", "breadbug", "--title", "Report a problem", "breadbug"])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open a terminal: {error}"))
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(name);
                candidate.is_file()
                    && std::os::unix::fs::PermissionsExt::mode(
                        &std::fs::metadata(&candidate).unwrap().permissions(),
                    ) & 0o111
                        != 0
            })
        })
        .unwrap_or(false)
}
