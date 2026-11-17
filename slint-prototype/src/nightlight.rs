//! Night light persistence and the Always-on hyprsunset path used by Displays.

use std::path::PathBuf;

use toml_edit::{value, DocumentMut, Item, Table};

pub const DEFAULT_TEMP: u32 = 3500;

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub enabled: bool,
    pub temperature: u32,
    /// `always`, `sunset` or `custom`.
    pub mode: String,
    /// Warm period for the custom schedule, as `HH:MM`.
    pub start: String,
    pub end: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: DEFAULT_TEMP,
            mode: "always".into(),
            start: "22:00".into(),
            end: "07:00".into(),
        }
    }
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn path() -> PathBuf {
    config_dir().join("bos-settings/nightlight.toml")
}

fn autostart_path() -> PathBuf {
    config_dir().join("autostart/hyprsunset.desktop")
}

pub fn load() -> Config {
    load_from(&path())
}

pub fn load_from(path: &std::path::Path) -> Config {
    let mut cfg = Config::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return cfg;
    };
    let Ok(doc) = text.parse::<DocumentMut>() else {
        return cfg;
    };
    if let Some(enabled) = doc.get("enabled").and_then(Item::as_bool) {
        cfg.enabled = enabled;
    }
    if let Some(temperature) = doc.get("temperature").and_then(Item::as_integer) {
        cfg.temperature = clamp_temp(temperature as u32);
    }
    if let Some(mode) = doc.get("mode").and_then(Item::as_str) {
        if ["always", "sunset", "custom"].contains(&mode) {
            cfg.mode = mode.to_string();
        }
    }
    if let Some(start) = doc.get("start").and_then(Item::as_str) {
        cfg.start = start.to_string();
    }
    if let Some(end) = doc.get("end").and_then(Item::as_str) {
        cfg.end = end.to_string();
    }
    cfg
}

pub struct Status {
    pub installed: bool,
    pub running: bool,
    pub enabled: bool,
    pub temperature: u32,
    pub mode: String,
    pub start: String,
    pub end: String,
}

pub fn status() -> Status {
    let cfg = load();
    Status {
        installed: crate::util::command_exists("hyprsunset"),
        running: std::process::Command::new("pgrep")
            .args(["-x", "hyprsunset"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false),
        enabled: cfg.enabled,
        temperature: cfg.temperature,
        mode: cfg.mode,
        start: cfg.start,
        end: cfg.end,
    }
}

pub fn apply(cfg: &Config) -> Result<Status, String> {
    save(cfg)?;
    Ok(status())
}

pub fn clamp_temp(temperature: u32) -> u32 {
    temperature.clamp(1000, 6500)
}

fn desktop_entry(temperature: u32) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Night light\nComment=Warm the screen after dark\nExec=hyprsunset -t {}\nX-GNOME-Autostart-enabled=true\n",
        clamp_temp(temperature)
    )
}

pub fn save(cfg: &Config) -> Result<(), String> {
    save_to(cfg, &path(), &autostart_path())
}

pub fn save_to(
    cfg: &Config,
    config_path: &std::path::Path,
    autostart: &std::path::Path,
) -> Result<(), String> {
    let mut doc = config_path
        .exists()
        .then(|| std::fs::read_to_string(config_path).ok())
        .flatten()
        .and_then(|text| text.parse::<DocumentMut>().ok())
        .unwrap_or_default();
    doc.insert("enabled", value(cfg.enabled));
    doc.insert("temperature", value(clamp_temp(cfg.temperature) as i64));
    doc.insert("mode", value(&cfg.mode));
    doc.insert("start", value(&cfg.start));
    doc.insert("end", value(&cfg.end));
    let table: Item = Item::Table(Table::new());
    let _ = table;
    write_atomic(config_path, &doc.to_string())?;
    if cfg.enabled {
        write_atomic(autostart, &desktop_entry(cfg.temperature))?;
        let _ = std::process::Command::new("pkill").args(["-x", "hyprsunset"]).status();
        std::process::Command::new("hyprsunset")
            .args(["-t", &clamp_temp(cfg.temperature).to_string()])
            .spawn()
            .map_err(|error| format!("hyprsunset: {error}"))?;
    } else {
        let _ = std::fs::remove_file(autostart);
        let _ = std::process::Command::new("pkill").args(["-x", "hyprsunset"]).status();
    }
    Ok(())
}

fn write_atomic(path: &std::path::Path, text: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!("{} has no parent", path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "nightlight".into()),
        std::process::id()
    ));
    std::fs::write(&temp, text).map_err(|error| format!("{}: {error}", temp.display()))?;
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("{}: {error}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_is_clamped() {
        assert_eq!(clamp_temp(100), 1000);
        assert_eq!(clamp_temp(9000), 6500);
        assert_eq!(clamp_temp(3000), 3000);
    }

    #[test]
    fn save_keeps_the_entry_and_autostart() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-night-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("nightlight.toml");
        let autostart = dir.join("hyprsunset.desktop");
        let cfg = Config {
            enabled: true,
            temperature: 3000,
            ..Config::default()
        };
        save_to(&cfg, &config_path, &autostart).unwrap();
        let text = std::fs::read_to_string(&config_path).unwrap();
        assert!(text.contains("enabled = true"));
        assert!(text.contains("temperature = 3000"));
        let entry = std::fs::read_to_string(&autostart).unwrap();
        assert!(entry.contains("Exec=hyprsunset -t 3000"));
        let loaded = load_from(&config_path);
        assert!(loaded.enabled);
        assert_eq!(loaded.temperature, 3000);
        let off = Config {
            enabled: false,
            temperature: 3000,
            ..Config::default()
        };
        save_to(&off, &config_path, &autostart).unwrap();
        assert!(!autostart.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
