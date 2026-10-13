//! Night light via hyprsunset (Hyprland twilight IPC). The compositor
//! talks to a hyprsunset daemon socket; if the binary is missing we offer
//! a pacman install rather than pretending the toggle works.
//!
//! Persistence goes through `hypr/autostart.json`, the Lua-era mechanism BOS
//! actually loads (`hyprland.lua` never reads `hyprland.conf`):
//!
//! * **Always on**: an autostart entry `hyprsunset -t <K>`, so the saved
//!   warmth comes back at login.
//! * **Sunset to sunrise / custom hours**: hyprsunset's own scheduler. Settings
//!   writes `~/.config/hypr/hyprsunset.conf` with two profiles (identity from
//!   the "off" time, `temperature` from the "on" time) and autostarts a bare
//!   `hyprsunset`, which reads that file. No extra daemon is needed.
//!
//! Sunset and sunrise are computed from the coordinates of the system time
//! zone (`zone.tab`), accurate to a few minutes. hyprsunset only understands
//! fixed clock times, so those times are recomputed whenever this panel
//! opens or changes and the daemon is restarted if they moved; a later
//! improvement would be a bread module that re-rolls them daily.

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::autostart::{self, AutostartEntry};
use super::config;
use super::util::{self, command_exists, fail_output};

const LEGACY_FRAGMENT: &str = "nightlight.conf";
const DEFAULT_TEMP: u32 = 3500;
const MIN_TEMP: u32 = 2000;
const MAX_TEMP: u32 = 6500;
const CONF_HEADER: &str =
    "# Managed by BOS Settings (Displays > Night light). Edits here are overwritten.\n";
const AUTOSTART_LABEL: &str = "Night light (hyprsunset, managed by Settings)";
const DEFAULT_START: &str = "21:00";
const DEFAULT_END: &str = "07:00";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Always,
    Custom,
    Sunset,
}

impl Mode {
    fn parse(s: &str) -> Option<Mode> {
        match s {
            "always" => Some(Mode::Always),
            "custom" => Some(Mode::Custom),
            "sunset" => Some(Mode::Sunset),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Mode::Always => "always",
            Mode::Custom => "custom",
            Mode::Sunset => "sunset",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NightlightConfig {
    enabled: bool,
    temperature: u32,
    mode: Mode,
    /// "HH:MM" the warm period starts (custom mode).
    start: String,
    /// "HH:MM" the warm period ends (custom mode).
    end: String,
}

impl Default for NightlightConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            temperature: DEFAULT_TEMP,
            mode: Mode::Sunset,
            start: DEFAULT_START.to_string(),
            end: DEFAULT_END.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct NightlightStatus {
    installed: bool,
    running: bool,
    enabled: bool,
    temperature: u32,
    mode: String,
    start: String,
    end: String,
    /// Today's computed sunrise/sunset ("HH:MM") and the zone they came from,
    /// when this machine's time zone has known coordinates.
    sunrise: Option<String>,
    sunset: Option<String>,
    location: Option<String>,
    error: Option<String>,
}

fn persist_path() -> std::path::PathBuf {
    util::bos_settings_dir().join("nightlight.toml")
}

fn load_persist() -> NightlightConfig {
    let Ok(text) = std::fs::read_to_string(persist_path()) else {
        return NightlightConfig::default();
    };
    parse_persist(&text)
}

fn parse_persist(text: &str) -> NightlightConfig {
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    let d = NightlightConfig::default();
    NightlightConfig {
        enabled: config::get_bool(&doc, &["enabled"]).unwrap_or(false),
        temperature: config::get_i64(&doc, &["temperature"])
            .unwrap_or(DEFAULT_TEMP as i64)
            .clamp(MIN_TEMP as i64, MAX_TEMP as i64) as u32,
        // Files written before schedules existed have no `mode`: they were
        // all-day, so keep that meaning for them.
        mode: config::get_str(&doc, &["mode"])
            .and_then(|m| Mode::parse(&m))
            .unwrap_or(Mode::Always),
        start: config::get_str(&doc, &["start"])
            .filter(|s| parse_hhmm(s).is_some())
            .unwrap_or(d.start),
        end: config::get_str(&doc, &["end"])
            .filter(|s| parse_hhmm(s).is_some())
            .unwrap_or(d.end),
    }
}

fn save_persist(cfg: &NightlightConfig) -> Result<(), String> {
    let mut doc = toml_edit::DocumentMut::new();
    config::set_bool(&mut doc, &["enabled"], cfg.enabled);
    config::set_i64(&mut doc, &["temperature"], cfg.temperature as i64);
    config::set_str(&mut doc, &["mode"], cfg.mode.as_str());
    config::set_str(&mut doc, &["start"], &cfg.start);
    config::set_str(&mut doc, &["end"], &cfg.end);
    let path = persist_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    config::atomic_write(&path, &doc.to_string()).map_err(|e| e.to_string())
}

fn clamp_temp(t: u32) -> u32 {
    t.clamp(MIN_TEMP, MAX_TEMP)
}

/// Strict "H:MM"/"HH:MM" (24-hour). Returns (hour, minute).
fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.trim().split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 {
        return None;
    }
    if !h.bytes().all(|b| b.is_ascii_digit()) || !m.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some((h, m))
}

fn fmt_hhmm(minutes: u32) -> String {
    let m = minutes % (24 * 60);
    format!("{:02}:{:02}", m / 60, m % 60)
}

// --- sunrise / sunset -------------------------------------------------------

/// ISO 6709 "+DDMM+DDDMM" or "+DDMMSS+DDDMMSS" (as in tzdata's zone.tab) to
/// (latitude, longitude) in degrees.
fn parse_iso6709(s: &str) -> Option<(f64, f64)> {
    let split = s[1..].find(['+', '-'])? + 1;
    let (lat, lon) = s.split_at(split);
    let part = |v: &str, deg_digits: usize| -> Option<f64> {
        let sign = match v.as_bytes().first()? {
            b'+' => 1.0,
            b'-' => -1.0,
            _ => return None,
        };
        let digits = &v[1..];
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let deg: f64 = digits.get(..deg_digits)?.parse().ok()?;
        let min: f64 = digits.get(deg_digits..deg_digits + 2)?.parse().ok()?;
        let sec: f64 = match digits.get(deg_digits + 2..) {
            Some("") | None => 0.0,
            Some(s) => s.parse().ok()?,
        };
        Some(sign * (deg + min / 60.0 + sec / 3600.0))
    };
    Some((part(lat, 2)?, part(lon, 3)?))
}

fn zone_coordinates(zone_tab: &str, tz: &str) -> Option<(f64, f64)> {
    zone_tab.lines().find_map(|line| {
        if line.starts_with('#') {
            return None;
        }
        let mut cols = line.split('\t');
        let _country = cols.next()?;
        let coords = cols.next()?;
        (cols.next()? == tz)
            .then(|| parse_iso6709(coords))
            .flatten()
    })
}

/// Sunrise and sunset as local minutes after midnight (Almanac for Computers
/// algorithm, zenith 90.833 degrees). `None` during polar day or night.
fn sun_times(
    lat: f64,
    lon: f64,
    year: i32,
    month: u32,
    day: u32,
    utc_offset_min: i32,
) -> Option<(u32, u32)> {
    let n1 = (275.0 * month as f64 / 9.0).floor();
    let n2 = ((month as f64 + 9.0) / 12.0).floor();
    let n3 = 1.0 + (((year - 4 * (year / 4)) as f64 + 2.0) / 3.0).floor();
    let n = n1 - n2 * n3 + day as f64 - 30.0;
    let lng_hour = lon / 15.0;
    let calc = |rising: bool| -> Option<f64> {
        let t = n + ((if rising { 6.0 } else { 18.0 }) - lng_hour) / 24.0;
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
        let cos_h =
            ((90.833 * rad).cos() - sin_dec * (lat * rad).sin()) / (cos_dec * (lat * rad).cos());
        if !(-1.0..=1.0).contains(&cos_h) {
            return None;
        }
        let h_deg = if rising {
            360.0 - cos_h.acos() / rad
        } else {
            cos_h.acos() / rad
        };
        let h = h_deg / 15.0;
        let t_local = h + ra - 0.06571 * t - 6.622;
        let ut = (t_local - lng_hour).rem_euclid(24.0);
        Some((ut + utc_offset_min as f64 / 60.0).rem_euclid(24.0))
    };
    let rise = calc(true)?;
    let set = calc(false)?;
    Some((
        (rise * 60.0).round() as u32 % 1440,
        (set * 60.0).round() as u32 % 1440,
    ))
}

/// Local calendar date and UTC offset (minutes) from `date`, avoiding a
/// time-zone crate for one lookup.
fn local_date_and_offset() -> Option<(i32, u32, u32, i32)> {
    let out = std::process::Command::new("date")
        .arg("+%Y %m %d %z")
        .output()
        .ok()?;
    parse_date_line(&String::from_utf8_lossy(&out.stdout))
}

fn parse_date_line(line: &str) -> Option<(i32, u32, u32, i32)> {
    let mut p = line.split_whitespace();
    let y: i32 = p.next()?.parse().ok()?;
    let m: u32 = p.next()?.parse().ok()?;
    let d: u32 = p.next()?.parse().ok()?;
    let z = p.next()?;
    let sign = match z.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hh: i32 = z.get(1..3)?.parse().ok()?;
    let mm: i32 = z.get(3..5)?.parse().ok()?;
    Some((y, m, d, sign * (hh * 60 + mm)))
}

fn current_timezone() -> Option<String> {
    let target = std::fs::read_link("/etc/localtime").ok()?;
    let target = target.to_string_lossy().into_owned();
    target.split_once("zoneinfo/").map(|(_, tz)| tz.to_string())
}

struct Sun {
    sunrise: String,
    sunset: String,
    zone: String,
}

fn todays_sun() -> Option<Sun> {
    let tz = current_timezone()?;
    let tab = std::fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()?;
    let (lat, lon) = zone_coordinates(&tab, &tz)?;
    let (y, m, d, off) = local_date_and_offset()?;
    let (rise, set) = sun_times(lat, lon, y, m, d, off)?;
    Some(Sun {
        sunrise: fmt_hhmm(rise),
        sunset: fmt_hhmm(set),
        zone: tz,
    })
}

// --- hyprsunset plumbing ----------------------------------------------------

/// The (warm starts, warm ends) pair for a scheduled mode; `None` for
/// always-on.
fn schedule_for(
    cfg: &NightlightConfig,
    sun: Option<&Sun>,
) -> Result<Option<(String, String)>, String> {
    match cfg.mode {
        Mode::Always => Ok(None),
        Mode::Custom => Ok(Some((cfg.start.clone(), cfg.end.clone()))),
        Mode::Sunset => match sun {
            Some(s) => Ok(Some((s.sunset.clone(), s.sunrise.clone()))),
            None => Err(
                "Can't work out sunset for this time zone. Use custom hours instead.".to_string(),
            ),
        },
    }
}

/// hyprsunset.conf: identity from `warm_end`, `temperature` from `warm_start`.
fn build_conf(warm_start: &str, warm_end: &str, temperature: u32) -> String {
    format!(
        "{CONF_HEADER}\nprofile {{\n    time = {warm_end}\n    identity = true\n}}\n\nprofile {{\n    time = {warm_start}\n    temperature = {temperature}\n}}\n"
    )
}

/// Command for the autostart entry: a fixed temperature, or bare hyprsunset
/// (which then follows hyprsunset.conf).
fn autostart_command(mode: Mode, temperature: u32) -> String {
    match mode {
        Mode::Always => format!("hyprsunset -t {}", clamp_temp(temperature)),
        _ => "hyprsunset".to_string(),
    }
}

fn is_hyprsunset_entry(e: &AutostartEntry) -> bool {
    e.command().split_whitespace().next() == Some("hyprsunset")
}

fn conf_path() -> std::path::PathBuf {
    util::hypr_dir().join("hyprsunset.conf")
}

/// Write our schedule file, refusing to clobber one the user wrote.
fn write_conf(warm_start: &str, warm_end: &str, temperature: u32) -> Result<(), String> {
    let path = conf_path();
    if let Ok(existing) = std::fs::read_to_string(&path) {
        if !existing.starts_with(CONF_HEADER) {
            return Err(
                "~/.config/hypr/hyprsunset.conf already exists and is not managed by Settings; \
                 move it aside to use a schedule."
                    .into(),
            );
        }
    }
    config::atomic_write(&path, &build_conf(warm_start, warm_end, temperature))
        .map_err(|e| e.to_string())
}

/// Remove hyprsunset.conf only when Settings wrote it.
fn remove_conf() {
    let path = conf_path();
    if std::fs::read_to_string(&path).is_ok_and(|t| t.starts_with(CONF_HEADER)) {
        let _ = std::fs::remove_file(path);
    }
}

fn set_autostart(mode: Mode, temperature: u32) -> Result<(), String> {
    let mut entries = autostart::load_entries();
    autostart::upsert_entry(
        &mut entries,
        is_hyprsunset_entry,
        AutostartEntry::new(autostart_command(mode, temperature), AUTOSTART_LABEL, true),
    );
    autostart::store_entries(entries)
}

fn clear_autostart() -> Result<(), String> {
    let mut entries = autostart::load_entries();
    if autostart::remove_entries(&mut entries, is_hyprsunset_entry) {
        autostart::store_entries(entries)?;
    }
    Ok(())
}

/// Older builds persisted through a hyprlang fragment that BOS never loads.
/// Tidy it up so the stale `source =` line does not linger.
fn clear_legacy_fragment() {
    let _ = std::fs::remove_file(util::hypr_dir().join(LEGACY_FRAGMENT));
    let _ = util::remove_hypr_source(LEGACY_FRAGMENT);
}

/// Liveness probe. `hyprctl hyprsunset gamma` takes a percentage (default
/// 100), so the value must be 100 here: `1.0` means 1% brightness and blacks
/// out the display.
async fn hyprsunset_running() -> bool {
    Command::new("hyprctl")
        .args(["hyprsunset", "gamma", "100"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn process_alive() -> bool {
    Command::new("pgrep")
        .args(["-x", "hyprsunset"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn stop_daemon() {
    let _ = Command::new("pkill")
        .args(["-x", "hyprsunset"])
        .status()
        .await;
    for _ in 0..10 {
        if !process_alive().await {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// (Re)start hyprsunset with the given arguments and wait for its socket.
async fn restart_daemon(args: &[String]) -> Result<(), String> {
    if !command_exists("hyprsunset") {
        return Err("hyprsunset is not installed".into());
    }
    stop_daemon().await;
    let mut child = std::process::Command::new("hyprsunset")
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("couldn't start hyprsunset: {e}"))?;
    // Reap it when it exits (turning night light off stops it); otherwise
    // every toggle left a <defunct> hyprsunset behind until the app quit.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    for _ in 0..15 {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if hyprsunset_running().await {
            return Ok(());
        }
    }
    Err("hyprsunset started but Hyprland twilight socket never came up".into())
}

async fn apply_temperature(temp: u32) -> Result<(), String> {
    let t = clamp_temp(temp).to_string();
    let output = Command::new("hyprctl")
        .args(["hyprsunset", "temperature", &t])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "hyprctl hyprsunset"))
    }
}

async fn apply_identity() -> Result<(), String> {
    if !hyprsunset_running().await {
        return Ok(());
    }
    let output = Command::new("hyprctl")
        .args(["hyprsunset", "identity"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "hyprctl hyprsunset"))
    }
}

async fn status_of(cfg: &NightlightConfig, error: Option<String>) -> NightlightStatus {
    let installed = command_exists("hyprsunset");
    let running = installed && hyprsunset_running().await;
    let sun = todays_sun();
    NightlightStatus {
        installed,
        running,
        enabled: cfg.enabled,
        temperature: cfg.temperature,
        mode: cfg.mode.as_str().to_string(),
        start: cfg.start.clone(),
        end: cfg.end.clone(),
        sunrise: sun.as_ref().map(|s| s.sunrise.clone()),
        sunset: sun.as_ref().map(|s| s.sunset.clone()),
        location: sun.map(|s| s.zone),
        error,
    }
}

#[tauri::command]
pub async fn get_nightlight() -> NightlightStatus {
    let cfg = load_persist();
    let mut error = None;
    // Sunset times drift through the year: when the saved schedule is a
    // computed one and today's differs, refresh the file and the daemon so
    // opening this panel never leaves a stale schedule behind.
    if cfg.enabled && cfg.mode == Mode::Sunset && command_exists("hyprsunset") {
        if let Some(sun) = todays_sun() {
            let wanted = build_conf(&sun.sunset, &sun.sunrise, cfg.temperature);
            let current = std::fs::read_to_string(conf_path()).unwrap_or_default();
            if current != wanted {
                match write_conf(&sun.sunset, &sun.sunrise, cfg.temperature) {
                    Ok(()) => {
                        if let Err(e) = restart_daemon(&[]).await {
                            error = Some(e);
                        }
                    }
                    Err(e) => error = Some(e),
                }
            }
        }
    }
    status_of(&cfg, error).await
}

#[tauri::command]
pub async fn set_nightlight(
    enabled: bool,
    temperature: u32,
    mode: Option<String>,
    start: Option<String>,
    end: Option<String>,
) -> Result<NightlightStatus, String> {
    let prev = load_persist();
    let mode = match mode.as_deref() {
        Some(m) => Mode::parse(m).ok_or_else(|| format!("unknown night light mode '{m}'"))?,
        None => prev.mode,
    };
    let valid_time = |t: Option<String>, fallback: &str| -> Result<String, String> {
        match t {
            Some(t) => parse_hhmm(&t)
                .map(|(h, m)| fmt_hhmm(h * 60 + m))
                .ok_or_else(|| format!("'{t}' is not a time (use HH:MM)")),
            None => Ok(fallback.to_string()),
        }
    };
    let mut cfg = NightlightConfig {
        enabled,
        temperature: clamp_temp(temperature),
        mode,
        start: valid_time(start, &prev.start)?,
        end: valid_time(end, &prev.end)?,
    };
    if !command_exists("hyprsunset") {
        cfg.enabled = false;
        return Ok(status_of(&cfg, Some("hyprsunset is not installed".into())).await);
    }

    let mut error: Option<String> = None;
    if cfg.enabled {
        match apply_enable(&cfg, &prev).await {
            Ok(()) => {}
            Err(e) => {
                error = Some(e);
                cfg.enabled = false;
            }
        }
    } else {
        let _ = apply_identity().await;
        stop_daemon().await;
        remove_conf();
        if let Err(e) = clear_autostart() {
            error = Some(e);
        }
    }
    clear_legacy_fragment();
    save_persist(&cfg)?;
    Ok(status_of(&cfg, error).await)
}

async fn apply_enable(cfg: &NightlightConfig, prev: &NightlightConfig) -> Result<(), String> {
    let sun = if cfg.mode == Mode::Sunset {
        todays_sun()
    } else {
        None
    };
    let schedule = schedule_for(cfg, sun.as_ref())?;
    // Persist first so a failed live apply still leaves the login behaviour
    // the user asked for.
    match &schedule {
        Some((warm_start, warm_end)) => write_conf(warm_start, warm_end, cfg.temperature)?,
        None => remove_conf(),
    }
    set_autostart(cfg.mode, cfg.temperature)?;

    let running = hyprsunset_running().await;
    let same_always = prev.enabled && prev.mode == Mode::Always && cfg.mode == Mode::Always;
    if cfg.mode == Mode::Always {
        if running && same_always {
            apply_temperature(cfg.temperature).await
        } else {
            restart_daemon(&["-t".to_string(), clamp_temp(cfg.temperature).to_string()]).await
        }
    } else {
        // The schedule lives in the config file, which hyprsunset only reads
        // at startup.
        restart_daemon(&[]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_clamps() {
        assert_eq!(clamp_temp(100), MIN_TEMP);
        assert_eq!(clamp_temp(9000), MAX_TEMP);
        assert_eq!(clamp_temp(3500), 3500);
    }

    #[test]
    fn hhmm_is_strict() {
        assert_eq!(parse_hhmm("21:00"), Some((21, 0)));
        assert_eq!(parse_hhmm("7:05"), Some((7, 5)));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("12:60"), None);
        assert_eq!(parse_hhmm("12:5"), None);
        assert_eq!(parse_hhmm("-1:00"), None);
        assert_eq!(parse_hhmm("12:00; reboot"), None);
        assert_eq!(parse_hhmm(""), None);
        assert_eq!(fmt_hhmm(7 * 60 + 5), "07:05");
    }

    #[test]
    fn autostart_command_carries_the_saved_temperature() {
        assert_eq!(autostart_command(Mode::Always, 3500), "hyprsunset -t 3500");
        assert_eq!(
            autostart_command(Mode::Always, 99_999),
            "hyprsunset -t 6500"
        );
        assert_eq!(autostart_command(Mode::Custom, 3500), "hyprsunset");
        assert_eq!(autostart_command(Mode::Sunset, 3500), "hyprsunset");
    }

    #[test]
    fn conf_has_an_identity_and_a_warm_profile() {
        let c = build_conf("21:00", "07:00", 3200);
        assert!(c.starts_with(CONF_HEADER));
        assert!(c.contains("time = 07:00\n    identity = true"));
        assert!(c.contains("time = 21:00\n    temperature = 3200"));
    }

    #[test]
    fn legacy_toml_without_mode_stays_all_day() {
        let cfg = parse_persist("enabled = true\ntemperature = 3000\n");
        assert!(cfg.enabled);
        assert_eq!(cfg.mode, Mode::Always);
        assert_eq!(cfg.temperature, 3000);
        let cfg = parse_persist(
            "enabled = true\nmode = \"custom\"\nstart = \"22:30\"\nend = \"06:15\"\n",
        );
        assert_eq!(cfg.mode, Mode::Custom);
        assert_eq!(cfg.start, "22:30");
        assert_eq!(cfg.end, "06:15");
        let bad = parse_persist("mode = \"nonsense\"\nstart = \"99:99\"\n");
        assert_eq!(bad.mode, Mode::Always);
        assert_eq!(bad.start, DEFAULT_START);
    }

    #[test]
    fn schedule_modes() {
        let mut cfg = NightlightConfig {
            enabled: true,
            mode: Mode::Always,
            ..Default::default()
        };
        assert_eq!(schedule_for(&cfg, None), Ok(None));
        cfg.mode = Mode::Custom;
        assert_eq!(
            schedule_for(&cfg, None),
            Ok(Some(("21:00".to_string(), "07:00".to_string())))
        );
        cfg.mode = Mode::Sunset;
        assert!(schedule_for(&cfg, None).is_err());
        let sun = Sun {
            sunrise: "05:41".into(),
            sunset: "18:03".into(),
            zone: "Australia/Perth".into(),
        };
        assert_eq!(
            schedule_for(&cfg, Some(&sun)),
            Ok(Some(("18:03".to_string(), "05:41".to_string())))
        );
    }

    #[test]
    fn iso6709_parses_tzdata_coordinates() {
        let (lat, lon) = parse_iso6709("-3157+11551").unwrap();
        assert!((lat + 31.95).abs() < 0.01 && (lon - 115.85).abs() < 0.01);
        let (lat, lon) = parse_iso6709("+513030-0000731").unwrap();
        assert!((lat - 51.508).abs() < 0.01 && (lon + 0.1253).abs() < 0.01);
        assert!(parse_iso6709("garbage").is_none());
        assert!(parse_iso6709("+3157").is_none());
    }

    #[test]
    fn zone_tab_lookup_ignores_comments() {
        let tab = "# comment\nAU\t-3157+11551\tAustralia/Perth\tWestern Australia\nGB\t+513030-0000731\tEurope/London\t\n";
        assert!(zone_coordinates(tab, "Australia/Perth").is_some());
        assert!(zone_coordinates(tab, "Europe/London").is_some());
        assert!(zone_coordinates(tab, "UTC").is_none());
    }

    fn within(actual: u32, hh: u32, mm: u32, tol: i32) -> bool {
        (actual as i32 - (hh * 60 + mm) as i32).abs() <= tol
    }

    #[test]
    fn sun_times_match_known_values() {
        // Perth, 1 Oct 2026 (UTC+8, a week past the equinox, sun at about -3
        // deg declination): day length about 12 h 20 m including refraction,
        // centred on solar noon about 12:07 (116 deg E is 17 minutes behind the
        // 120 deg meridian, minus ~10 min equation of time).
        let (rise, set) = sun_times(-31.95, 115.85, 2026, 10, 1, 8 * 60).unwrap();
        let len = set - rise;
        assert!(
            (12 * 60 + 12..12 * 60 + 30).contains(&len),
            "day length {len}"
        );
        assert!(
            within((rise + set) / 2, 12, 7, 6),
            "noon {}",
            fmt_hhmm((rise + set) / 2)
        );
        // London, 21 Jun 2026 (BST, UTC+1): about 04:43 and 21:21.
        let (rise, set) = sun_times(51.508, -0.1253, 2026, 6, 21, 60).unwrap();
        assert!(within(rise, 4, 43, 10), "rise {}", fmt_hhmm(rise));
        assert!(within(set, 21, 21, 10), "set {}", fmt_hhmm(set));
    }

    #[test]
    fn polar_night_has_no_sunrise() {
        // Tromso in late December never sees the sun rise.
        assert!(sun_times(69.65, 18.96, 2026, 12, 21, 60).is_none());
    }

    #[test]
    fn date_line_parses_offsets() {
        assert_eq!(
            parse_date_line("2026 10 01 +0800\n"),
            Some((2026, 10, 1, 480))
        );
        assert_eq!(
            parse_date_line("2026 01 15 -0330"),
            Some((2026, 1, 15, -210))
        );
        assert_eq!(parse_date_line("nope"), None);
    }

    #[test]
    fn only_hyprsunset_entries_are_ours() {
        assert!(is_hyprsunset_entry(&AutostartEntry::new(
            "hyprsunset -t 3000",
            "",
            true
        )));
        assert!(is_hyprsunset_entry(&AutostartEntry::new(
            "hyprsunset",
            "",
            true
        )));
        assert!(!is_hyprsunset_entry(&AutostartEntry::new(
            "hyprsunsetx",
            "",
            true
        )));
        assert!(!is_hyprsunset_entry(&AutostartEntry::new(
            "breadbar", "", true
        )));
    }
}
