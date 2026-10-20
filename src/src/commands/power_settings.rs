//! Fixed power profile commands and non-destructive idle configuration edits.
use super::{config, util};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerSettings {
    pub dim: u32,
    pub screen_off: u32,
    pub lock: u32,
    pub sleep: u32,
    pub sleep_on_battery: bool,
    pub lid: String,
    pub power_button: String,
}
impl Default for PowerSettings {
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
#[derive(Serialize)]
pub struct PowerModes {
    backend: String,
    current: String,
    profiles: Vec<String>,
    /// TLP is installed but its power-profile service (`tlp-pd`, which
    /// provides `tlpctl`) is not, so modes can be set up with one install.
    tlp_setup_available: bool,
}
async fn command_text(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().await.ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().into())
}
#[tauri::command]
pub async fn get_power_modes() -> PowerModes {
    for backend in ["powerprofilesctl", "tlpctl"] {
        if let Some(list) = command_text(backend, &["list"]).await {
            let profiles = ["power-saver", "balanced", "performance"]
                .into_iter()
                .filter(|p| list.contains(p))
                .map(String::from)
                .collect::<Vec<_>>();
            if !profiles.is_empty() {
                return PowerModes {
                    backend: backend.into(),
                    current: command_text(backend, &["get"]).await.unwrap_or_default(),
                    profiles,
                    tlp_setup_available: false,
                };
            }
        }
    }
    PowerModes {
        backend: String::new(),
        current: String::new(),
        profiles: vec![],
        tlp_setup_available: util::command_exists("tlp") && !util::command_exists("tlpctl"),
    }
}

/// Starts TLP's power-profile service after `tlp-pd` is installed (through
/// the allow-listed `pacman_install`). Fixed unit, no caller-supplied argv.
#[tauri::command]
pub async fn enable_tlp_power_profiles() -> Result<(), String> {
    if !util::command_exists("tlpctl") {
        return Err("Install tlp-pd first".into());
    }
    let out = Command::new("pkexec")
        .args(["systemctl", "enable", "--now", "tlp-pd.service"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Turning on power modes"))
    }
}
#[tauri::command]
pub async fn set_power_mode(profile: String) -> Result<(), String> {
    if get_automatic_power_mode().await {
        return Err("Turn off automatic switching before selecting a manual profile".into());
    }
    let modes = get_power_modes().await;
    if !modes.profiles.contains(&profile) {
        return Err("This power profile is not available".into());
    }
    let out = Command::new(modes.backend)
        .args(["set", &profile])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Changing power mode"))
    }
}
const BATTERY_SLEEP: &str =
    "if ! grep -q '^1$' /sys/class/power_supply/*/online 2>/dev/null; then systemctl suspend; fi";
/// Screen off/on through the Lua API. BOS configures stock Hyprland with
/// hyprland.lua; in that mode `hyprctl dispatch dpms off` is parsed as Lua
/// and fails (exit 7), so idle
/// screen-off written that way never fired.
const DPMS_OFF: &str = "hyprctl eval \"hl.dispatch(hl.dsp.dpms({ action = 'off' }))\"";
const DPMS_ON: &str = "hyprctl eval \"hl.dispatch(hl.dsp.dpms({ action = 'on' }))\"";
/// Rewrites the legacy dispatch spelling (BOS's own skel shipped it) to the
/// working one, so reading and saving both see the same command.
fn migrate_dpms(text: &str) -> String {
    text.replace("hyprctl dispatch dpms off", DPMS_OFF)
        .replace("hyprctl dispatch dpms on", DPMS_ON)
        .replace(LEGACY_DIM, DIM)
        .replace(LEGACY_UNDIM, UNDIM)
}
/// Plain `brightnessctl` acts on the first device it finds, a keyboard LED
/// on machines without a backlight (BSET-73), so dimming names the class.
const DIM: &str = "brightnessctl -c backlight -s set 10%";
const UNDIM: &str = "brightnessctl -c backlight -r";
const LEGACY_DIM: &str = "brightnessctl -s set 10%";
const LEGACY_UNDIM: &str = "brightnessctl -r";
const ACTIONS: [(&str, &str); 4] = [
    ("dim", DIM),
    ("screen_off", DPMS_OFF),
    ("lock", "loginctl lock-session"),
    ("sleep", "systemctl suspend"),
];
fn idle_regex() -> regex::Regex {
    // Quoted strings may hold braces: the Lua dpms command does.
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
/// The shipped skel line (`pidof breadlock || breadlock`) never starts a
/// locker while `breadlock listen` is running. Replace only that exact
/// command so a hand-written lock_cmd is left alone.
fn migrate_lock_cmd(text: &str) -> String {
    text.replace(
        "lock_cmd = pidof breadlock || breadlock",
        &format!("lock_cmd = {LOCK_CMD}"),
    )
    .replace(
        "lock_cmd         = pidof breadlock || breadlock",
        &format!("lock_cmd         = {LOCK_CMD}"),
    )
}
fn timeout_of(text: &str, action: &str) -> Option<u32> {
    let text = migrate_dpms(text);
    idle_regex()
        .find_iter(&text)
        .find(|m| matches_action(m.as_str(), action))
        .and_then(|m| timeout_regex().captures(m.as_str()))
        .and_then(|c| c[2].parse().ok())
}
/// Matches a running locker only. `pidof breadlock` is also true for the
/// always-running `breadlock listen`, which made lock_cmd a no-op.
const LOCK_CMD: &str = "pgrep -xf breadlock >/dev/null || breadlock";
fn render_idle(text: &str, p: &PowerSettings) -> String {
    let mut result = if text.trim().is_empty() {
        format!("general {{\n lock_cmd = {LOCK_CMD}\n before_sleep_cmd = loginctl lock-session\n after_sleep_cmd = {DPMS_ON}\n ignore_dbus_inhibit = false\n}}\n")
    } else {
        migrate_lock_cmd(&migrate_dpms(text))
    };
    for ((_, action), seconds) in ACTIONS.iter().zip([p.dim, p.screen_off, p.lock, p.sleep]) {
        let mut found = false;
        result = idle_regex()
            .replace_all(&result, |c: &regex::Captures| {
                let block = &c[0];
                if !matches_action(block, action) {
                    return block.to_string();
                }
                found = true;
                if seconds == 0 {
                    return String::new();
                };
                let changed = timeout_regex()
                    .replace(block, |t: &regex::Captures| format!("{}{seconds}", &t[1]))
                    .into_owned();
                if *action == "systemctl suspend" {
                    let command = if p.sleep_on_battery {
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
                a if a == DPMS_OFF => format!("\n on-resume = {DPMS_ON}"),
                a if a == DIM => format!("\n on-resume = {UNDIM}"),
                _ => String::new(),
            };
            let action = if *action == "systemctl suspend" && p.sleep_on_battery {
                BATTERY_SLEEP
            } else {
                action
            };
            result.push_str(&format!("\n# Managed by BOS Settings\nlistener {{\n timeout = {seconds}\n on-timeout = {action}{resume}\n}}\n"))
        }
    }
    result
}
fn validate(p: &PowerSettings) -> Result<(), String> {
    if [p.dim, p.screen_off, p.lock, p.sleep]
        .into_iter()
        .any(|v| v > 86400)
        || !["suspend", "lock", "ignore"].contains(&p.lid.as_str())
        || !["menu", "suspend", "poweroff", "ignore"].contains(&p.power_button.as_str())
    {
        Err("Invalid power settings".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
pub fn get_power_settings() -> PowerSettings {
    let mut p = PowerSettings::default();
    let path = config::config_dir().join("hypr/hypridle.conf");
    if let Ok(text) = std::fs::read_to_string(path) {
        p.dim = timeout_of(&text, ACTIONS[0].1).unwrap_or(0);
        p.screen_off = timeout_of(&text, ACTIONS[1].1).unwrap_or(0);
        p.lock = timeout_of(&text, ACTIONS[2].1).unwrap_or(0);
        p.sleep = timeout_of(&text, ACTIONS[3].1).unwrap_or(0);
        p.sleep_on_battery = idle_regex()
            .find_iter(&text)
            .any(|m| matches_action(m.as_str(), BATTERY_SLEEP));
    }
    if let Ok(text) = std::fs::read_to_string("/etc/systemd/logind.conf.d/99-bos-settings.conf") {
        for line in text.lines() {
            if let Some(s) = line.strip_prefix("HandleLidSwitch=") {
                p.lid = s.into()
            }
            if let Some(s) = line.strip_prefix("HandlePowerKey=") {
                p.power_button = s.into()
            }
        }
    }
    if std::fs::read_to_string(config::config_dir().join("hypr/bos-settings-power-button.lua"))
        .is_ok_and(|s| s.contains("-- BOS power menu"))
    {
        p.power_button = "menu".into();
    }
    p
}
#[tauri::command]
pub async fn save_power_settings(settings: PowerSettings) -> Result<(), String> {
    validate(&settings)?;
    let current = get_power_settings();
    if settings.lid != current.lid || settings.power_button != current.power_button {
        let out = Command::new("pkexec")
            .args(["install", "-d", "-m", "0755", "/etc/systemd/logind.conf.d"])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(util::fail_output(
                &out,
                "Creating power configuration directory",
            ));
        }
        let body=format!("# Managed by BOS Settings. Takes effect after restart.\n[Login]\nHandleLidSwitch={}\nHandlePowerKey={}\n",settings.lid,if settings.power_button=="menu" {"ignore"} else {settings.power_button.as_str()});
        if !util::run_with_stdin(
            &[
                "pkexec",
                "tee",
                "/etc/systemd/logind.conf.d/99-bos-settings.conf",
            ],
            &body,
        )
        .await
        {
            return Err("Power button and lid settings were not saved".into());
        }
    }
    let fragment = if settings.power_button == "menu" {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let path = exe.to_str().ok_or("Executable path is not UTF-8")?;
        if path.contains(['\n', '\r', '\0']) {
            return Err("Invalid executable path".into());
        }
        let shell = format!("'{}' --page power-menu", path.replace('\'', "'\\''"));
        format!(
            "-- BOS power menu\nhl.bind('XF86PowerOff', hl.dsp.exec_cmd({}))\n",
            serde_json::to_string(&shell).map_err(|e| e.to_string())?
        )
    } else {
        "-- BOS Settings: power button handled by logind\n".into()
    };
    super::desktop_preferences::install_lua_include("bos-settings-power-button.lua", &fragment)?;
    util::hypr_reload();
    let path = config::config_dir().join("hypr/hypridle.conf");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    config::atomic_write(&path, &render_idle(&text, &settings)).map_err(|e| e.to_string())?;
    // Apply to the next idle daemon start. Reloading the session is not required.
    let stop = Command::new("pkill")
        .args(["-x", "hypridle"])
        .status()
        .await;
    if stop.is_ok() && util::command_exists("hypridle") {
        std::process::Command::new("hypridle")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Saved, but could not restart the idle daemon: {e}"))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_dpms_dispatch_is_read_and_rewritten_for_lua() {
        // BOS's skel hypridle.conf: fails when Hyprland runs from hyprland.lua.
        let text = "general {\n after_sleep_cmd = hyprctl dispatch dpms on\n}\nlistener {\n timeout = 330\n on-timeout = hyprctl dispatch dpms off\n on-resume  = hyprctl dispatch dpms on\n}\n";
        assert_eq!(timeout_of(text, DPMS_OFF), Some(330));
        let p = PowerSettings {
            screen_off: 600,
            ..Default::default()
        };
        let rendered = render_idle(text, &p);
        assert!(!rendered.contains("dispatch dpms"), "{rendered}");
        assert!(rendered.contains("hl.dsp.dpms({ action = 'off' })"));
        assert_eq!(timeout_of(&rendered, DPMS_OFF), Some(600));
    }
    #[test]
    fn battery_only_sleep_and_custom_command_are_preserved() {
        let text="listener {\n timeout = 42\n on-timeout = echo systemctl suspend\n}\nlistener {\n timeout = 300\n on-timeout = systemctl suspend\n}\n";
        let p = PowerSettings {
            sleep: 120,
            ..Default::default()
        };
        let rendered = render_idle(text, &p);
        assert!(rendered.contains("on-timeout = echo systemctl suspend"));
        assert!(rendered.contains(BATTERY_SLEEP));
        assert_eq!(timeout_of(&rendered, "systemctl suspend"), Some(120));
        let p = PowerSettings {
            sleep_on_battery: false,
            ..p
        };
        let rendered = render_idle(&rendered, &p);
        assert!(!rendered.contains(BATTERY_SLEEP));
    }
    #[test]
    fn preserve_custom_idle_actions() {
        let text="# my comment\nlistener {\n timeout = 42\n on-timeout = custom-app\n}\nlistener {\n timeout = 300\n on-timeout = loginctl lock-session\n}\n";
        let p = PowerSettings {
            lock: 120,
            dim: 0,
            screen_off: 0,
            sleep: 0,
            ..Default::default()
        };
        let result = render_idle(text, &p);
        assert!(result.contains("my comment"));
        assert!(result.contains("timeout = 42"));
        assert_eq!(timeout_of(&result, "loginctl lock-session"), Some(120));
    }
    #[test]
    fn dim_targets_the_backlight_and_migrates_old_lines() {
        let old = "listener {\n timeout = 300\n on-timeout = brightnessctl -s set 10%\n on-resume = brightnessctl -r\n}\n";
        assert_eq!(timeout_of(old, DIM), Some(300));
        let p = PowerSettings {
            dim: 120,
            ..Default::default()
        };
        let out = render_idle(old, &p);
        assert!(out.contains("on-timeout = brightnessctl -c backlight -s set 10%"));
        assert!(out.contains("on-resume = brightnessctl -c backlight -r"));
        assert!(!out.contains("on-timeout = brightnessctl -s set"));
        assert_eq!(timeout_of(&out, DIM), Some(120));
        assert_eq!(render_idle(&out, &p), out, "migration is idempotent");
        let fresh = render_idle("", &p);
        assert!(fresh.contains("on-resume = brightnessctl -c backlight -r"));
    }
    #[test]
    fn broken_pidof_lock_cmd_is_rewritten_and_custom_lock_cmd_is_kept() {
        let skel = "general {\n    lock_cmd         = pidof breadlock || breadlock\n    before_sleep_cmd = loginctl lock-session\n}\n";
        let out = render_idle(skel, &PowerSettings::default());
        assert!(out.contains("lock_cmd         = pgrep -xf breadlock >/dev/null || breadlock"), "{out}");
        assert!(!out.contains("pidof breadlock"), "{out}");
        let again = render_idle(&out, &PowerSettings::default());
        assert_eq!(again, out, "migration is idempotent");
        let custom = "general {\n lock_cmd = swaylock\n}\n";
        assert!(render_idle(custom, &PowerSettings::default()).contains("lock_cmd = swaylock"));
        let fresh = render_idle("", &PowerSettings::default());
        assert!(fresh.contains("lock_cmd = pgrep -xf breadlock >/dev/null || breadlock"));
        assert!(!fresh.contains("pidof"));
    }
    #[test]
    fn never_removes_managed_listener() {
        let p = PowerSettings {
            dim: 0,
            ..Default::default()
        };
        let result = render_idle(&render_idle("", &PowerSettings::default()), &p);
        assert!(!result.contains("brightnessctl -c backlight -s set"));
        assert!(validate(&PowerSettings {
            lid: ";reboot".into(),
            ..Default::default()
        })
        .is_err());
    }
}

#[tauri::command]
pub async fn get_automatic_power_mode() -> bool {
    Command::new("systemctl")
        .args(["is-enabled", "bos-settings-power-auto.timer"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
fn automatic_profile_script(backend: &str) -> Result<String, String> {
    if !["powerprofilesctl", "tlpctl"].contains(&backend) {
        return Err("Unsupported profile service".into());
    }
    Ok(format!("#!/bin/sh\n# Managed by BOS Settings\nac=0\nlow=0\nfor supply in /sys/class/power_supply/*; do\n [ -f \"$supply/type\" ] || continue\n type=$(cat \"$supply/type\")\n if [ \"$type\" = Battery ]; then\n  capacity=$(cat \"$supply/capacity\" 2>/dev/null)\n  case \"$capacity\" in ''|*[!0-9]*) ;; *) [ \"$capacity\" -lt 20 ] && low=1 ;; esac\n elif [ -f \"$supply/online\" ] && [ \"$(cat \"$supply/online\")\" = 1 ]; then\n  ac=1\n fi\ndone\nprofile=balanced\n[ \"$low\" = 1 ] && profile=power-saver\n[ \"$ac\" = 1 ] && profile=performance\ncurrent=$(/usr/bin/{backend} get) || exit 1\n[ \"$current\" = \"$profile\" ] && exit 0\nexec /usr/bin/{backend} set \"$profile\"\n"))
}
#[tauri::command]
pub async fn set_automatic_power_mode(enabled: bool) -> Result<(), String> {
    if !enabled {
        let out = Command::new("pkexec")
            .args([
                "systemctl",
                "disable",
                "--now",
                "bos-settings-power-auto.timer",
            ])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        return if out.status.success() {
            Ok(())
        } else {
            Err(util::fail_output(&out, "Disabling automatic power mode"))
        };
    }
    let modes = get_power_modes().await;
    if !["power-saver", "balanced", "performance"]
        .iter()
        .all(|p| modes.profiles.iter().any(|v| v == p))
    {
        return Err("Automatic switching needs all three power profiles".into());
    }
    let profile_script = automatic_profile_script(&modes.backend)?;
    let service="[Unit]\nDescription=BOS automatic power profile\n[Service]\nType=oneshot\nExecStart=/usr/lib/bos-settings-power-auto\n";
    let timer="[Unit]\nDescription=Choose BOS power profile by power source and charge\n[Timer]\nOnBootSec=30s\nOnUnitActiveSec=30s\n[Install]\nWantedBy=timers.target\n";
    let mut install = String::from("set -eu\n");
    for (path, content) in [
        ("/usr/lib/bos-settings-power-auto", profile_script.as_str()),
        (
            "/etc/systemd/system/bos-settings-power-auto.service",
            service,
        ),
        ("/etc/systemd/system/bos-settings-power-auto.timer", timer),
    ] {
        install.push_str(&format!("if [ -f '{path}' ]; then cp -p '{path}' '{path}.bak'; fi\ncat > '{path}' <<'BOS_SETTINGS_UNIT'\n{content}BOS_SETTINGS_UNIT\n"));
    }
    install.push_str("chmod 0755 /usr/lib/bos-settings-power-auto\nchmod 0644 /etc/systemd/system/bos-settings-power-auto.service /etc/systemd/system/bos-settings-power-auto.timer\nsystemctl daemon-reload\nsystemctl enable --now bos-settings-power-auto.timer\nsystemctl start bos-settings-power-auto.service\n");
    if util::run_with_stdin(&["pkexec", "sh", "-s"], &install).await {
        Ok(())
    } else {
        Err("Automatic power switching was not enabled. Check authentication and the profile service.".into())
    }
}
#[cfg(test)]
mod automatic_tests {
    use super::*;
    #[test]
    fn script_only_uses_fixed_profile_programs() {
        assert!(automatic_profile_script("tlpctl;reboot").is_err());
        let s = automatic_profile_script("powerprofilesctl").unwrap();
        assert!(s.contains("-lt 20"));
        assert!(s.contains("profile=performance"));
        assert!(s.contains("/usr/bin/powerprofilesctl set"));
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    Sleep,
    Restart,
    PowerOff,
}
#[tauri::command]
pub async fn run_power_action(action: PowerAction) -> Result<(), String> {
    let command = match action {
        PowerAction::Sleep => "suspend",
        PowerAction::Restart => "reboot",
        PowerAction::PowerOff => "poweroff",
    };
    let out = Command::new("systemctl")
        .arg(command)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Power action"))
    }
}
