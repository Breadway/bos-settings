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

fn valid_app_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(*byte, b'.' | b'_' | b'-')
        })
}

/// Validate and normalise an exclusion list: valid app ids only, sorted,
/// without duplicates.
pub fn clean_exclusions(apps: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for app in apps {
        if !valid_app_id(app) {
            return Err(format!("'{app}' is not a valid app id"));
        }
        out.push(app.clone());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn save_exclusions(apps: &[String]) -> Result<(), String> {
    let clean = clean_exclusions(apps)?;
    let path = config_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut doc: toml_edit::DocumentMut = text.parse().map_err(|error| format!("{error}"))?;
    let mut array = toml_edit::Array::new();
    for app in &clean {
        array.push(app.as_str());
    }
    doc["excluded_apps"] = toml_edit::value(array);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, doc.to_string()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_are_valid_sorted_and_deduplicated() {
        let list = clean_exclusions(&["search".into(), "clip".into(), "clip".into()]).unwrap();
        assert_eq!(list, ["clip", "search"]);
        for bad in ["../x", "Clip", "a b", "", "x;rm"] {
            assert!(
                clean_exclusions(&[bad.to_string()]).is_err(),
                "{bad:?} should be refused"
            );
        }
    }
}

pub struct Report {
    pub when: u64,
    pub kind: String,
    pub app: String,
    pub status: String,
    pub path: Option<String>,
    pub receipt: Option<String>,
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
            receipt: value
                .get("receipt")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
        .collect();
    rows.reverse();
    rows
}

fn endpoint() -> String {
    let text = std::fs::read_to_string(config_path()).unwrap_or_default();
    text.parse::<toml_edit::DocumentMut>()
        .ok()
        .and_then(|doc| doc.get("endpoint").and_then(|item| item.as_str()).map(str::to_string))
        .filter(|value| value.starts_with("https://") || value.starts_with("http://"))
        .unwrap_or_else(|| "https://bugs.breadway.dev".into())
}

/// Deletes a sent report through the server, then drops it from history.
/// Needs sending enabled, like every network action in breadbug.
pub fn delete_report(receipt: &str) -> Result<String, String> {
    if !load().upload {
        return Err("Sending is turned off, so breadbug will not contact the server. Turn it on first.".into());
    }
    if receipt.len() != 32 || !receipt.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("That is not a report receipt.".into());
    }
    let path = state_dir().join("history.jsonl");
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut token = None;
    for line in text.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("receipt").and_then(|v| v.as_str()) == Some(receipt) {
            token = value
                .get("deletion_token")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            break;
        }
    }
    let token = token.ok_or("This report has no deletion token on this computer.")?;
    let url = format!("{}/v1/reports/{receipt}", endpoint().trim_end_matches('/'));
    let output = Command::new("curl")
        .args([
            "-sS",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-X",
            "DELETE",
            "-H",
            &format!("Authorization: Bearer {token}"),
            &url,
        ])
        .output()
        .map_err(|error| format!("could not reach the server: {error}"))?;
    let code: u16 = String::from_utf8_lossy(&output.stdout).trim().parse().unwrap_or(0);
    let found = match code {
        200 | 204 => true,
        404 => false,
        _ => {
            return Err(format!(
                "The server refused the delete ({code}). {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    };
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|value| {
                    value
                        .get("receipt")
                        .and_then(|v| v.as_str())
                        .map(|value| value != receipt)
                })
                .unwrap_or(true)
        })
        .collect();
    let mut body = kept.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    std::fs::write(&path, body).map_err(|error| error.to_string())?;
    Ok(if found {
        "Deleted from the server.".into()
    } else {
        "The server no longer had it. Removed from your history.".into()
    })
}

pub fn store_bytes() -> u64 {
    fn walk(dir: &std::path::Path) -> u64 {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| {
                        let path = entry.path();
                        if path.is_dir() {
                            walk(&path)
                        } else {
                            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
    walk(&state_dir().join("bugstore"))
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
