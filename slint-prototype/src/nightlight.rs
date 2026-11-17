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

const CONF_HEADER: &str =
    "# Managed by BOS Settings (Displays > Night light). Edits here are overwritten.\n";

fn conf_path() -> PathBuf {
    config_dir().join("hypr/hyprsunset.conf")
}

fn fmt_hhmm(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60 % 24, minutes % 60)
}

fn parse_iso6709(text: &str) -> Option<(f64, f64)> {
    let split = text[1..].find(['+', '-'])? + 1;
    let (latitude, longitude) = text.split_at(split);
    let part = |value: &str, degree_digits: usize| -> Option<f64> {
        let sign = match value.as_bytes().first()? {
            b'+' => 1.0,
            b'-' => -1.0,
            _ => return None,
        };
        let digits = &value[1..];
        if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let degrees: f64 = digits.get(..degree_digits)?.parse().ok()?;
        let minutes: f64 = digits.get(degree_digits..degree_digits + 2)?.parse().ok()?;
        let seconds: f64 = match digits.get(degree_digits + 2..) {
            Some("") | None => 0.0,
            Some(rest) => rest.parse().ok()?,
        };
        Some(sign * (degrees + minutes / 60.0 + seconds / 3600.0))
    };
    Some((part(latitude, 2)?, part(longitude, 3)?))
}

fn zone_coordinates(zone_tab: &str, zone: &str) -> Option<(f64, f64)> {
    zone_tab.lines().find_map(|line| {
        if line.starts_with('#') {
            return None;
        }
        let mut columns = line.split('\t');
        columns.next()?;
        let coordinates = columns.next()?;
        (columns.next()? == zone)
            .then(|| parse_iso6709(coordinates))
            .flatten()
    })
}

fn sun_times(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    utc_offset_min: i32,
) -> Option<(u32, u32)> {
    let n1 = (275.0 * month as f64 / 9.0).floor();
    let n2 = ((month as f64 + 9.0) / 12.0).floor();
    let n3 = 1.0 + (((year - 4 * (year / 4)) as f64 + 2.0) / 3.0).floor();
    let n = n1 - n2 * n3 + day as f64 - 30.0;
    let longitude_hour = longitude / 15.0;
    let calculate = |rising: bool| -> Option<f64> {
        let t = n + ((if rising { 6.0 } else { 18.0 }) - longitude_hour) / 24.0;
        let m = 0.9856 * t - 3.289;
        let rad = std::f64::consts::PI / 180.0;
        let mut l = m + 1.916 * (m * rad).sin() + 0.020 * (2.0 * m * rad).sin() + 282.634;
        l = l.rem_euclid(360.0);
        let mut ra = (0.91764 * (l * rad).tan()).atan() / rad;
        ra = ra.rem_euclid(360.0);
        ra += (l / 90.0).floor() * 90.0 - (ra / 90.0).floor() * 90.0;
        ra /= 15.0;
        let sin_dec = 0.39782 * (l * rad).sin();
        let cos_dec = sin_dec.asin().cos();
        let cos_h = ((90.833 * rad).cos() - sin_dec * (latitude * rad).sin())
            / (cos_dec * (latitude * rad).cos());
        if !(-1.0..=1.0).contains(&cos_h) {
            return None;
        }
        let h_degrees = if rising {
            360.0 - cos_h.acos() / rad
        } else {
            cos_h.acos() / rad
        };
        let h = h_degrees / 15.0;
        let local = h + ra - 0.06571 * t - 6.622;
        let ut = (local - longitude_hour).rem_euclid(24.0);
        Some((ut + utc_offset_min as f64 / 60.0).rem_euclid(24.0))
    };
    let rise = calculate(true)?;
    let set = calculate(false)?;
    Some((
        (rise * 60.0).round() as u32 % 1440,
        (set * 60.0).round() as u32 % 1440,
    ))
}

fn local_date_and_offset() -> Option<(i32, u32, u32, i32)> {
    let out = std::process::Command::new("date")
        .arg("+%Y %m %d %z")
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&out.stdout);
    let mut parts = line.split_whitespace();
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    let zone = parts.next()?;
    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours: i32 = zone.get(1..3)?.parse().ok()?;
    let minutes: i32 = zone.get(3..5)?.parse().ok()?;
    Some((year, month, day, sign * (hours * 60 + minutes)))
}

fn current_timezone() -> Option<String> {
    let target = std::fs::read_link("/etc/localtime").ok()?;
    let text = target.to_string_lossy().into_owned();
    text.split_once("zoneinfo/")
        .map(|(_, zone)| zone.to_string())
}

pub struct Sun {
    pub sunrise: String,
    pub sunset: String,
    pub zone: String,
}

/// Today's sunrise and sunset where the system's time zone is.
pub fn todays_sun() -> Option<Sun> {
    let zone = current_timezone()?;
    let table = std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()?;
    let (latitude, longitude) = zone_coordinates(&table, &zone)?;
    let (year, month, day, offset) = local_date_and_offset()?;
    let (rise, set) = sun_times(latitude, longitude, year, month, day, offset)?;
    Some(Sun {
        sunrise: fmt_hhmm(rise),
        sunset: fmt_hhmm(set),
        zone,
    })
}

fn schedule_for(cfg: &Config) -> Result<Option<(String, String)>, String> {
    match cfg.mode.as_str() {
        "always" => Ok(None),
        "custom" => Ok(Some((cfg.start.clone(), cfg.end.clone()))),
        "sunset" => match todays_sun() {
            Some(sun) => Ok(Some((sun.sunset, sun.sunrise))),
            None => Err(
                "Can't work out sunset for this time zone. Use custom hours instead.".into(),
            ),
        },
        _ => Err("Unknown night light schedule".into()),
    }
}

fn build_conf(warm_start: &str, warm_end: &str, temperature: u32) -> String {
    format!(
        "{CONF_HEADER}\nprofile {{\n    time = {warm_end}\n    identity = true\n}}\n\nprofile {{\n    time = {warm_start}\n    temperature = {temperature}\n}}\n"
    )
}

fn desktop_entry_for(cfg: &Config) -> String {
    let command = if schedule_for(cfg).ok().flatten().is_some() {
        "hyprsunset".to_string()
    } else {
        format!("hyprsunset -t {}", clamp_temp(cfg.temperature))
    };
    format!(
        "[Desktop Entry]\nType=Application\nName=Night light\nComment=Warm the screen after dark\nExec={command}\nX-GNOME-Autostart-enabled=true\n"
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
    let conf = conf_path();
    let schedule = if cfg.enabled {
        schedule_for(cfg)?
    } else {
        None
    };
    if let Some((start, end)) = &schedule {
        if std::fs::read_to_string(&conf).is_ok_and(|text| !text.starts_with(CONF_HEADER)) {
            return Err(
                "~/.config/hypr/hyprsunset.conf already exists and is not managed by Settings; move it aside to use a schedule."
                    .into(),
            );
        }
        write_atomic(&conf, &build_conf(start, end, cfg.temperature))?;
    } else if std::fs::read_to_string(&conf).is_ok_and(|text| text.starts_with(CONF_HEADER)) {
        let _ = std::fs::remove_file(&conf);
    }
    if cfg.enabled {
        write_atomic(autostart, &desktop_entry_for(cfg))?;
        let _ = std::process::Command::new("pkill").args(["-x", "hyprsunset"]).status();
        let mut command = std::process::Command::new("hyprsunset");
        if schedule.is_none() {
            command.args(["-t", &clamp_temp(cfg.temperature).to_string()]);
        }
        command
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
