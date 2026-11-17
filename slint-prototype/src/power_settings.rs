//! Idle policy, lid and power button handling, automatic power profiles, and
//! the hardware battery protection switch.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util;

pub const BATTERY_SLEEP: &str =
    "if ! grep -q '^1$' /sys/class/power_supply/*/online 2>/dev/null; then systemctl suspend; fi";
const DPMS_OFF: &str = "hyprctl eval \"hl.dispatch(hl.dsp.dpms({ action = 'off' }))\"";
const DPMS_ON: &str = "hyprctl eval \"hl.dispatch(hl.dsp.dpms({ action = 'on' }))\"";
const DIM: &str = "brightnessctl -c backlight -s set 10%";
const UNDIM: &str = "brightnessctl -c backlight -r";
const LEGACY_DIM: &str = "brightnessctl -s set 10%";
const LEGACY_UNDIM: &str = "brightnessctl -r";
const LOCK_CMD: &str = "pgrep -xf breadlock >/dev/null || breadlock";
const ACTIONS: [(&str, &str); 4] = [
    ("dim", DIM),
    ("screen_off", DPMS_OFF),
    ("lock", "loginctl lock-session"),
    ("sleep", "systemctl suspend"),
];

#[derive(Clone)]
pub struct Settings {
    pub dim: u32,
    pub screen_off: u32,
    pub lock: u32,
    pub sleep: u32,
    pub sleep_on_battery: bool,
    pub lid: String,
    pub power_button: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dim: 300,
            screen_off: 480,
            lock: 510,
            sleep: 1200,
            sleep_on_battery: true,
            lid: "suspend".into(),
            power_button: "poweroff".into(),
        }
    }
}

fn idle_file() -> PathBuf {
    crate::config::config_dir().join("hypr/hypridle.conf")
}

fn logind_file() -> PathBuf {
    PathBuf::from("/etc/systemd/logind.conf.d/99-bos-settings.conf")
}

fn power_button_lua() -> PathBuf {
    crate::config::config_dir().join("hypr/bos-settings-power-button.lua")
}

fn migrate(text: &str) -> String {
    let text = text
        .replace("hyprctl dispatch dpms off", DPMS_OFF)
        .replace("hyprctl dispatch dpms on", DPMS_ON)
        .replace(LEGACY_DIM, DIM)
        .replace(LEGACY_UNDIM, UNDIM);
    let lock = regex::Regex::new(
        r"(?m)^([ \t]*lock_cmd)([ \t]*)=([ \t]*)pidof breadlock \|\| breadlock([ \t]*(?:#.*)?)[ \t]*$",
    )
    .unwrap();
    lock.replace_all(&text, |captures: &regex::Captures| {
        format!("{}{}={}{}", &captures[1], &captures[2], &captures[3], LOCK_CMD)
    })
    .into_owned()
}

fn idle_regex() -> regex::Regex {
    regex::Regex::new(r#"(?s)listener\s*\{(?:[^{}"]|"[^"]*")*\}"#).unwrap()
}

fn timeout_regex() -> regex::Regex {
    regex::Regex::new(r"(?m)^(\s*timeout\s*=\s*)(\d+)").unwrap()
}

fn matches_action(block: &str, action: &str) -> bool {
    block.lines().any(|line| {
        line.split_once('=').is_some_and(|(key, value)| {
            key.trim() == "on-timeout"
                && (value.trim() == action
                    || (action == "systemctl suspend" && value.trim() == BATTERY_SLEEP))
        })
    })
}

fn timeout_of(text: &str, action: &str) -> Option<u32> {
    let text = migrate(text);
    idle_regex()
        .find_iter(&text)
        .find(|found| matches_action(found.as_str(), action))
        .and_then(|found| timeout_regex().captures(found.as_str()))
        .and_then(|captures| captures[2].parse().ok())
}

pub fn get() -> Settings {
    let mut settings = Settings::default();
    if let Ok(text) = std::fs::read_to_string(idle_file()) {
        settings.dim = timeout_of(&text, ACTIONS[0].1).unwrap_or(0);
        settings.screen_off = timeout_of(&text, ACTIONS[1].1).unwrap_or(0);
        settings.lock = timeout_of(&text, ACTIONS[2].1).unwrap_or(0);
        settings.sleep = timeout_of(&text, ACTIONS[3].1).unwrap_or(0);
        settings.sleep_on_battery = idle_regex()
            .find_iter(&text)
            .any(|found| matches_action(found.as_str(), BATTERY_SLEEP));
    }
    if let Ok(text) = std::fs::read_to_string(logind_file()) {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("HandleLidSwitch=") {
                settings.lid = value.into();
            }
            if let Some(value) = line.strip_prefix("HandlePowerKey=") {
                settings.power_button = value.into();
            }
        }
    }
    if std::fs::read_to_string(power_button_lua())
        .is_ok_and(|text| text.contains("-- BOS power menu"))
    {
        settings.power_button = "menu".into();
    }
    settings
}

fn validate(settings: &Settings) -> Result<(), String> {
    if [settings.dim, settings.screen_off, settings.lock, settings.sleep]
        .into_iter()
        .any(|value| value > 86400)
        || !["suspend", "lock", "ignore"].contains(&settings.lid.as_str())
        || !["menu", "suspend", "poweroff", "ignore"].contains(&settings.power_button.as_str())
    {
        Err("Invalid power settings".into())
    } else {
        Ok(())
    }
}

pub fn render_idle(text: &str, settings: &Settings) -> String {
    let mut result = if text.trim().is_empty() {
        format!("general {{\n lock_cmd = {LOCK_CMD}\n before_sleep_cmd = loginctl lock-session\n after_sleep_cmd = {DPMS_ON}\n ignore_dbus_inhibit = false\n}}\n")
    } else {
        migrate(text)
    };
    for ((_, action), seconds) in ACTIONS
        .iter()
        .zip([settings.dim, settings.screen_off, settings.lock, settings.sleep])
    {
        let mut found = false;
        result = idle_regex()
            .replace_all(&result, |captures: &regex::Captures| {
                let block = &captures[0];
                if !matches_action(block, action) {
                    return block.to_string();
                }
                found = true;
                if seconds == 0 {
                    return String::new();
                }
                let changed = timeout_regex()
                    .replace(block, |timeout: &regex::Captures| {
                        format!("{}{seconds}", &timeout[1])
                    })
                    .into_owned();
                if *action == "systemctl suspend" {
                    let command = if settings.sleep_on_battery {
                        BATTERY_SLEEP
                    } else {
                        action
                    };
                    changed
                        .lines()
                        .map(|line| {
                            if line.trim_start().starts_with("on-timeout") {
                                format!(" on-timeout = {command}")
                            } else {
                                line.into()
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("\n")
                } else {
                    changed
                }
            })
            .into_owned();
        if !found && seconds > 0 {
            let resume = match *action {
                action if action == DPMS_OFF => format!("\n on-resume = {DPMS_ON}"),
                action if action == DIM => format!("\n on-resume = {UNDIM}"),
                _ => String::new(),
            };
            let action = if *action == "systemctl suspend" && settings.sleep_on_battery {
                BATTERY_SLEEP
            } else {
                action
            };
            result.push_str(&format!(
                "\n# Managed by BOS Settings\nlistener {{\n timeout = {seconds}\n on-timeout = {action}{resume}\n}}\n"
            ));
        }
    }
    result
}

pub fn save(settings: &Settings) -> Result<(), String> {
    validate(settings)?;
    let current = get();
    if settings.lid != current.lid || settings.power_button != current.power_button {
        let _ = Command::new("pkexec")
            .args(["install", "-d", "-m", "0755", "/etc/systemd/logind.conf.d"])
            .output();
        let body = format!(
            "# Managed by BOS Settings. Takes effect after restart.\n[Login]\nHandleLidSwitch={}\nHandlePowerKey={}\n",
            settings.lid,
            if settings.power_button == "menu" {
                "ignore"
            } else {
                settings.power_button.as_str()
            }
        );
        if !util::run_with_stdin(
            &[
                "pkexec",
                "tee",
                "/etc/systemd/logind.conf.d/99-bos-settings.conf",
            ],
            &body,
        ) {
            return Err("Power button and lid settings were not saved".into());
        }
    }
    let fragment = if settings.power_button == "menu" {
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let path = exe.to_str().ok_or("Executable path is not UTF-8")?;
        if path.contains(['\n', '\r', '\0']) {
            return Err("Invalid executable path".into());
        }
        let shell = format!("'{}' --page power-menu", path.replace('\'', "'\\''"));
        format!(
            "-- BOS power menu\nhl.bind('XF86PowerOff', hl.dsp.exec_cmd({}))\n",
            serde_json::to_string(&shell).map_err(|error| error.to_string())?
        )
    } else {
        "-- BOS Settings: power button handled by logind\n".into()
    };
    let lua = power_button_lua();
    crate::config::atomic_write(&lua, &fragment).map_err(|error| error.to_string())?;
    let _ = util::ensure_hypr_source("bos-settings-power-button.lua");
    util::hypr_reload();
    let path = idle_file();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    crate::config::atomic_write(&path, &render_idle(&text, settings))
        .map_err(|error| error.to_string())?;
    let stop = Command::new("pkill").args(["-x", "hypridle"]).status();
    if stop.is_ok() && util::command_exists("hypridle") {
        Command::new("hypridle")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| format!("Saved, but could not restart the idle daemon: {error}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------- automatic

pub fn automatic_enabled() -> bool {
    Command::new("systemctl")
        .args(["is-enabled", "bos-settings-power-auto.timer"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn automatic_script(backend: &str) -> Result<String, String> {
    if !["powerprofilesctl", "tlpctl"].contains(&backend) {
        return Err("Unsupported profile service".into());
    }
    Ok(format!("#!/bin/sh\n# Managed by BOS Settings\nac=0\nlow=0\nfor supply in /sys/class/power_supply/*; do\n [ -f \"$supply/type\" ] || continue\n type=$(cat \"$supply/type\")\n if [ \"$type\" = Battery ]; then\n  capacity=$(cat \"$supply/capacity\" 2>/dev/null)\n  case \"$capacity\" in ''|*[!0-9]*) ;; *) [ \"$capacity\" -lt 20 ] && low=1 ;; esac\n elif [ -f \"$supply/online\" ] && [ \"$(cat \"$supply/online\")\" = 1 ]; then\n  ac=1\n fi\ndone\nprofile=balanced\n[ \"$low\" = 1 ] && profile=power-saver\n[ \"$ac\" = 1 ] && profile=performance\ncurrent=$(/usr/bin/{backend} get) || exit 1\n[ \"$current\" = \"$profile\" ] && exit 0\nexec /usr/bin/{backend} set \"$profile\"\n"))
}

pub fn set_automatic(enabled: bool) -> Result<(), String> {
    if !enabled {
        let out = Command::new("pkexec")
            .args([
                "systemctl",
                "disable",
                "--now",
                "bos-settings-power-auto.timer",
            ])
            .output()
            .map_err(|error| error.to_string())?;
        return if out.status.success() {
            Ok(())
        } else {
            Err(util::fail_output(&out, "Disabling automatic power mode"))
        };
    }
    let backend = ["powerprofilesctl", "tlpctl"]
        .into_iter()
        .find(|program| util::command_exists(program))
        .ok_or("No power profile service is installed")?;
    let script = automatic_script(backend)?;
    let service="[Unit]\nDescription=BOS automatic power profile\n[Service]\nType=oneshot\nExecStart=/usr/lib/bos-settings-power-auto\n";
    let timer="[Unit]\nDescription=Choose BOS power profile by power source and charge\n[Timer]\nOnBootSec=30s\nOnUnitActiveSec=30s\n[Install]\nWantedBy=timers.target\n";
    let mut install = String::from("set -eu\n");
    for (path, content) in [
        ("/usr/lib/bos-settings-power-auto", script.as_str()),
        (
            "/etc/systemd/system/bos-settings-power-auto.service",
            service,
        ),
        ("/etc/systemd/system/bos-settings-power-auto.timer", timer),
    ] {
        install.push_str(&format!("if [ -f '{path}' ]; then cp -p '{path}' '{path}.bak'; fi\ncat > '{path}' <<'BOS_SETTINGS_UNIT'\n{content}BOS_SETTINGS_UNIT\n"));
    }
    install.push_str("chmod 0755 /usr/lib/bos-settings-power-auto\nchmod 0644 /etc/systemd/system/bos-settings-power-auto.service /etc/systemd/system/bos-settings-power-auto.timer\nsystemctl daemon-reload\nsystemctl enable --now bos-settings-power-auto.timer\nsystemctl start bos-settings-power-auto.service\n");
    if util::run_with_stdin(&["pkexec", "sh", "-s"], &install) {
        Ok(())
    } else {
        Err("Automatic power switching was not enabled. Check authentication and the profile service.".into())
    }
}

pub fn enable_tlp_power_profiles() -> Result<(), String> {
    if !util::command_exists("tlpctl") {
        return Err("Install tlp-pd first".into());
    }
    let out = Command::new("pkexec")
        .args(["systemctl", "enable", "--now", "tlp-pd.service"])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Turning on power modes"))
    }
}

pub fn run_power_action(action: &str) -> Result<(), String> {
    let command = match action {
        "sleep" => "suspend",
        "restart" => "reboot",
        "power-off" | "poweroff" => "poweroff",
        _ => return Err("unknown power action".into()),
    };
    let out = Command::new("systemctl")
        .arg(command)
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Power action"))
    }
}

// ---------------------------------------------------------------- protection

const PROTECT_STOP_PCT: i64 = 80;
const PROTECT_RESUME_PCT: i64 = 75;
const IDEAPAD_DRIVER_DIR: &str = "/sys/bus/platform/drivers/ideapad_acpi";

#[derive(Clone, Debug, PartialEq)]
enum Protection {
    Thresholds { start: PathBuf, end: PathBuf },
    ChargeTypes(PathBuf),
    Conservation(PathBuf),
}

pub struct ProtectionInfo {
    pub kind: &'static str,
    pub enabled: bool,
    pub limit_pct: Option<i64>,
}

fn parse_charge_types(text: &str) -> (Option<String>, Vec<String>) {
    let mut active = None;
    let mut all = Vec::new();
    for token in text.split_whitespace() {
        if let Some(inner) = token.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            active = Some(inner.to_string());
            all.push(inner.to_string());
        } else {
            all.push(token.to_string());
        }
    }
    (active, all)
}

fn detect_protection_in(power_supply: &Path, ideapad: &Path) -> Option<Protection> {
    let mut batteries: Vec<PathBuf> = std::fs::read_dir(power_supply)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    batteries.sort();
    for dir in &batteries {
        let start = dir.join("charge_control_start_threshold");
        let end = dir.join("charge_control_end_threshold");
        if end.exists() {
            return Some(Protection::Thresholds { start, end });
        }
    }
    for dir in &batteries {
        let types = dir.join("charge_types");
        if let Ok(text) = std::fs::read_to_string(&types) {
            let (_, all) = parse_charge_types(&text);
            if all.iter().any(|kind| kind == "Long_Life") {
                return Some(Protection::ChargeTypes(types));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(ideapad) {
        let mut found: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path().join("conservation_mode"))
            .filter(|path| path.is_file())
            .collect();
        found.sort();
        if let Some(path) = found.into_iter().next() {
            return Some(Protection::Conservation(path));
        }
    }
    None
}

fn detect_protection() -> Option<Protection> {
    detect_protection_in(
        Path::new("/sys/class/power_supply"),
        Path::new(IDEAPAD_DRIVER_DIR),
    )
}

fn valid_protect_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    if text.contains("..") || text.contains('\n') || text.contains('\0') || text.contains(' ') {
        return false;
    }
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    let under_power_supply = text.starts_with("/sys/class/power_supply/");
    let under_ideapad = text.starts_with(&format!("{IDEAPAD_DRIVER_DIR}/"));
    match name {
        "charge_control_start_threshold" | "charge_control_end_threshold" | "charge_types" => {
            under_power_supply
        }
        "conservation_mode" => under_ideapad,
        _ => false,
    }
}

fn read_threshold(path: &Path) -> i64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(100)
}

fn read_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
}

fn protection_enabled(protection: &Protection) -> bool {
    match protection {
        Protection::Thresholds { end, .. } => read_threshold(end) < 100,
        Protection::ChargeTypes(path) => read_trimmed(path)
            .map(|text| parse_charge_types(&text).0.as_deref() == Some("Long_Life"))
            .unwrap_or(false),
        Protection::Conservation(path) => read_trimmed(path).as_deref() == Some("1"),
    }
}

fn protect_writes(protection: &Protection, enable: bool) -> Vec<(PathBuf, String)> {
    match protection {
        Protection::Thresholds { start, end } => {
            let (start_pct, end_pct) = if enable {
                (PROTECT_RESUME_PCT, PROTECT_STOP_PCT)
            } else {
                (0, 100)
            };
            let mut writes = Vec::new();
            if enable {
                writes.push((end.clone(), format!("{end_pct}\n")));
                if start.exists() {
                    writes.push((start.clone(), format!("{start_pct}\n")));
                }
            } else {
                if start.exists() {
                    writes.push((start.clone(), format!("{start_pct}\n")));
                }
                writes.push((end.clone(), format!("{end_pct}\n")));
            }
            writes
        }
        Protection::ChargeTypes(path) => vec![(
            path.clone(),
            if enable { "Long_Life\n" } else { "Standard\n" }.to_string(),
        )],
        Protection::Conservation(path) => {
            vec![(path.clone(), if enable { "1\n" } else { "0\n" }.to_string())]
        }
    }
}

pub fn protection_info() -> Option<ProtectionInfo> {
    let protection = detect_protection()?;
    let enabled = protection_enabled(&protection);
    Some(match &protection {
        Protection::Thresholds { end, .. } => ProtectionInfo {
            kind: "thresholds",
            enabled,
            limit_pct: Some(read_threshold(end)),
        },
        Protection::ChargeTypes(_) => ProtectionInfo {
            kind: "charge_types",
            enabled,
            limit_pct: None,
        },
        Protection::Conservation(_) => ProtectionInfo {
            kind: "conservation",
            enabled,
            limit_pct: None,
        },
    })
}

pub fn set_battery_protection(enabled: bool) -> Result<ProtectionInfo, String> {
    let Some(protection) = detect_protection() else {
        return Err("This battery has no protection mode to switch.".into());
    };
    for (path, value) in protect_writes(&protection, enabled) {
        if !valid_protect_path(&path) {
            return Err("Refusing to write to an unexpected battery path.".into());
        }
        if !util::run_with_stdin(&["pkexec", "tee", &path.display().to_string()], &value) {
            return Err(
                "Couldn't change battery protection (authentication cancelled or refused).".into(),
            );
        }
    }
    protection_info().ok_or_else(|| "Battery protection disappeared.".to_string())
}

fn charge_threshold_paths() -> Option<(PathBuf, PathBuf)> {
    let base = Path::new("/sys/class/power_supply");
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        let start = entry.path().join("charge_control_start_threshold");
        let end = entry.path().join("charge_control_end_threshold");
        if start.exists() && end.exists() {
            return Some((start, end));
        }
    }
    None
}

pub fn set_charge_threshold(which: &str, percent: i64) -> Result<(), String> {
    if which != "start" && which != "end" {
        return Err("threshold must be start or end".into());
    }
    let percent = percent.clamp(0, 100);
    let Some((start, end)) = charge_threshold_paths() else {
        return Err("No charge threshold support on this hardware".into());
    };
    let path = if which == "start" { start } else { end };
    if !valid_protect_path(&path) {
        return Err("Refusing to write to an unexpected battery path.".into());
    }
    if util::run_with_stdin(
        &["pkexec", "tee", &path.display().to_string()],
        &format!("{percent}\n"),
    ) {
        Ok(())
    } else {
        Err("Failed to set charge threshold".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_listener_is_removed_for_never() {
        let text = "listener {\n timeout = 300\n on-timeout = brightnessctl -c backlight -s set 10%\n}\n";
        assert_eq!(timeout_of(text, DIM), Some(300));
        let settings = Settings {
            dim: 0,
            ..Settings::default()
        };
        let rendered = render_idle(text, &settings);
        assert!(!rendered.contains("brightnessctl -c backlight -s set"));
    }

    #[test]
    fn legacy_dpms_is_rewritten() {
        let text = "listener {\n timeout = 330\n on-timeout = hyprctl dispatch dpms off\n}\n";
        assert_eq!(timeout_of(text, DPMS_OFF), Some(330));
        let rendered = render_idle(text, &Settings::default());
        assert!(rendered.contains("hl.dsp.dpms"));
        assert!(!rendered.contains("dispatch dpms"));
    }

    #[test]
    fn new_listener_is_added_once() {
        let settings = Settings {
            lock: 120,
            dim: 0,
            screen_off: 0,
            sleep: 0,
            ..Settings::default()
        };
        let once = render_idle("", &settings);
        assert!(once.contains("loginctl lock-session"));
        assert_eq!(render_idle(&once, &settings), once);
    }

    #[test]
    fn invalid_values_are_refused() {
        assert!(validate(&Settings {
            lid: ";reboot".into(),
            ..Settings::default()
        })
        .is_err());
        assert!(automatic_script("tlpctl;reboot").is_err());
    }
}
