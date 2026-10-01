//! Battery/power status (upower), brightness (brightnessctl), and TLP's
//! current profile. Deliberately no AC/Battery/Performance *switcher*: TLP
//! automatically picks a profile by power source, so this only exposes
//! controls for things that are actually user choices: brightness, and
//! charge thresholds where the hardware supports them.

use serde::Serialize;
use tokio::process::Command;

use super::util;

async fn upower_device(kind: &str) -> Option<String> {
    let out = Command::new("upower").arg("-e").output().await.ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.to_lowercase().contains(kind))
        .map(str::to_string)
}

async fn upower_field(device: &str, field: &str) -> Option<String> {
    let out = Command::new("upower")
        .args(["-i", device])
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.trim_start().starts_with(field))
        .and_then(|l| l.split(':').nth(1))
        .map(|v| v.trim().to_string())
}

async fn battery_summary() -> Vec<(String, String)> {
    let Some(bat) = upower_device("bat").await else {
        return vec![("Battery".to_string(), "No battery detected".to_string())];
    };
    let mut rows = Vec::new();
    if let Some(state) = upower_field(&bat, "state").await {
        rows.push(("Status".to_string(), state));
    }
    if let Some(pct) = upower_field(&bat, "percentage").await {
        rows.push(("Charge".to_string(), pct));
    }
    let t = match upower_field(&bat, "time to empty").await {
        Some(t) => Some(t),
        None => upower_field(&bat, "time to full").await,
    };
    if let Some(t) = t {
        rows.push(("Time remaining".to_string(), t));
    }
    let full: Option<f64> = upower_field(&bat, "energy-full")
        .await
        .and_then(|v| v.split_whitespace().next()?.parse().ok());
    let design: Option<f64> = upower_field(&bat, "energy-full-design")
        .await
        .and_then(|v| v.split_whitespace().next()?.parse().ok());
    if let (Some(full), Some(design)) = (full, design) {
        if design > 0.0 {
            rows.push((
                "Battery health".to_string(),
                format!("{:.0}% of design capacity", full / design * 100.0),
            ));
        }
    }
    rows
}

async fn power_source() -> String {
    match upower_device("ac").await {
        Some(ac) => match upower_field(&ac, "online").await {
            Some(v) if v == "yes" => "AC power".to_string(),
            Some(_) => "Battery".to_string(),
            None => "Unknown".to_string(),
        },
        None => "Unknown".to_string(),
    }
}

async fn tlp_profile() -> Option<String> {
    let out = Command::new("tlp-stat").arg("-s").output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.trim_start().starts_with("TLP profile"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().to_string())
}

async fn brightness_device() -> Option<String> {
    let out = Command::new("brightnessctl").output().await.ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with("Device"))
        .and_then(|l| l.split('\'').nth(1))
        .map(str::to_string)
}

async fn brightness_pct() -> Option<u32> {
    let out = Command::new("brightnessctl").output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| l.contains("Current brightness"))
        .and_then(|l| l.split('(').nth(1))
        .and_then(|v| v.trim_end_matches("%)").parse().ok())
}

/// Charge-threshold sysfs paths, only Some when the running kernel driver
/// actually exposes them: genuinely hardware-dependent, not every install
/// will have this.
fn charge_threshold_paths() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let base = std::path::Path::new("/sys/class/power_supply");
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let start = entry.path().join("charge_control_start_threshold");
        let end = entry.path().join("charge_control_end_threshold");
        if start.exists() && end.exists() {
            return Some((start, end));
        }
    }
    None
}

fn read_threshold(path: &std::path::Path) -> i64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(100)
}

// --- "Protect battery" -------------------------------------------------------
//
// Laptops expose battery protection three different ways. One switch hides
// them, preferring the most capable mechanism the hardware has:
//
// 1. `charge_control_{start,end}_threshold` (ThinkPad, ASUS, Framework, ...):
//    real limits; protect = stop at 80% and resume at 75%.
// 2. `charge_types` listing `Long_Life` (ideapad_laptop on kernel 6.x, some
//    ASUS/Huawei): a fixed protection mode, usually about 80%.
// 3. `conservation_mode` (older ideapad_laptop, Lenovo IdeaPad/Yoga): a 0/1
//    switch for the same fixed mode.
//
// Writes need root, so they go through the `pkexec tee <path>` pattern used by
// `set_charge_threshold`. The path is never taken from the webview: it is
// found here under /sys and re-validated by [`valid_protect_path`] right
// before the write, and the value is chosen by [`protect_writes`].

const PROTECT_STOP_PCT: i64 = 80;
const PROTECT_RESUME_PCT: i64 = 75;
const IDEAPAD_DRIVER_DIR: &str = "/sys/bus/platform/drivers/ideapad_acpi";

#[derive(Clone, Debug, PartialEq)]
enum Protection {
    Thresholds {
        start: std::path::PathBuf,
        end: std::path::PathBuf,
    },
    ChargeTypes(std::path::PathBuf),
    Conservation(std::path::PathBuf),
}

/// `charge_types` reads like `Standard [Long_Life]` (the active type is
/// bracketed) and lists every type the battery supports.
fn parse_charge_types(text: &str) -> (Option<String>, Vec<String>) {
    let mut active = None;
    let mut all = Vec::new();
    for tok in text.split_whitespace() {
        if let Some(inner) = tok.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            active = Some(inner.to_string());
            all.push(inner.to_string());
        } else {
            all.push(tok.to_string());
        }
    }
    (active, all)
}

fn detect_protection_in(
    power_supply: &std::path::Path,
    ideapad_dir: &std::path::Path,
) -> Option<Protection> {
    let mut batteries: Vec<std::path::PathBuf> = std::fs::read_dir(power_supply)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .collect();
    batteries.sort();
    for dir in &batteries {
        let start = dir.join("charge_control_start_threshold");
        let end = dir.join("charge_control_end_threshold");
        if end.exists() {
            // Some drivers only have an end threshold; start is optional.
            return Some(Protection::Thresholds { start, end });
        }
    }
    for dir in &batteries {
        let types = dir.join("charge_types");
        if let Ok(text) = std::fs::read_to_string(&types) {
            let (_, all) = parse_charge_types(&text);
            if all.iter().any(|t| t == "Long_Life") {
                return Some(Protection::ChargeTypes(types));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(ideapad_dir) {
        let mut found: Vec<_> = entries
            .flatten()
            .map(|e| e.path().join("conservation_mode"))
            .filter(|p| p.is_file())
            .collect();
        found.sort();
        if let Some(p) = found.into_iter().next() {
            return Some(Protection::Conservation(p));
        }
    }
    None
}

fn detect_protection() -> Option<Protection> {
    detect_protection_in(
        std::path::Path::new("/sys/class/power_supply"),
        std::path::Path::new(IDEAPAD_DRIVER_DIR),
    )
}

/// Only these sysfs files may ever be written through pkexec.
fn valid_protect_path(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    if s.contains("..") || s.contains('\n') || s.contains('\0') || s.contains(' ') {
        return false;
    }
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let under_ps = s.starts_with("/sys/class/power_supply/");
    let under_ideapad = s.starts_with(&format!("{IDEAPAD_DRIVER_DIR}/"));
    match name {
        "charge_control_start_threshold" | "charge_control_end_threshold" | "charge_types" => {
            under_ps
        }
        "conservation_mode" => under_ideapad,
        _ => false,
    }
}

fn read_trimmed(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn protection_enabled(p: &Protection) -> bool {
    match p {
        Protection::Thresholds { end, .. } => read_threshold(end) < 100,
        Protection::ChargeTypes(path) => read_trimmed(path)
            .map(|t| parse_charge_types(&t).0.as_deref() == Some("Long_Life"))
            .unwrap_or(false),
        Protection::Conservation(path) => read_trimmed(path).as_deref() == Some("1"),
    }
}

/// The ordered (path, value) writes that turn protection on or off. Order
/// matters for thresholds: never leave start above end.
fn protect_writes(p: &Protection, enable: bool) -> Vec<(std::path::PathBuf, String)> {
    match p {
        Protection::Thresholds { start, end } => {
            let (s, e) = if enable {
                (PROTECT_RESUME_PCT, PROTECT_STOP_PCT)
            } else {
                (0, 100)
            };
            let mut v = Vec::new();
            if enable {
                v.push((end.clone(), format!("{e}\n")));
                if start.exists() {
                    v.push((start.clone(), format!("{s}\n")));
                }
            } else {
                if start.exists() {
                    v.push((start.clone(), format!("{s}\n")));
                }
                v.push((end.clone(), format!("{e}\n")));
            }
            v
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

#[derive(Serialize)]
pub struct BatteryProtection {
    /// `thresholds`, `charge_types` or `conservation`.
    kind: &'static str,
    enabled: bool,
    /// Where charging stops when protected, when the hardware says so.
    limit_pct: Option<i64>,
}

fn protection_info() -> Option<BatteryProtection> {
    let p = detect_protection()?;
    let enabled = protection_enabled(&p);
    Some(match &p {
        Protection::Thresholds { end, .. } => BatteryProtection {
            kind: "thresholds",
            enabled,
            limit_pct: Some(read_threshold(end)),
        },
        Protection::ChargeTypes(_) => BatteryProtection {
            kind: "charge_types",
            enabled,
            limit_pct: None,
        },
        Protection::Conservation(_) => BatteryProtection {
            kind: "conservation",
            enabled,
            limit_pct: None,
        },
    })
}

#[tauri::command]
pub async fn set_battery_protection(enabled: bool) -> Result<BatteryProtection, String> {
    let Some(p) = detect_protection() else {
        return Err("This battery has no protection mode to switch.".into());
    };
    for (path, value) in protect_writes(&p, enabled) {
        if !valid_protect_path(&path) {
            return Err("Refusing to write to an unexpected battery path.".into());
        }
        let ok =
            util::run_with_stdin(&["pkexec", "tee", &path.display().to_string()], &value).await;
        if !ok {
            return Err(
                "Couldn't change battery protection (authentication cancelled or refused).".into(),
            );
        }
    }
    protection_info().ok_or_else(|| "Battery protection disappeared.".to_string())
}

#[derive(Serialize)]
pub struct PowerInfo {
    battery: Vec<(String, String)>,
    power_source: String,
    brightness_pct: Option<u32>,
    charge_start: Option<i64>,
    charge_end: Option<i64>,
    tlp_profile: Option<String>,
    protection: Option<BatteryProtection>,
}

#[tauri::command]
pub async fn get_power_info() -> PowerInfo {
    let charge = charge_threshold_paths();
    PowerInfo {
        battery: battery_summary().await,
        power_source: power_source().await,
        brightness_pct: brightness_pct().await,
        charge_start: charge.as_ref().map(|(s, _)| read_threshold(s)),
        charge_end: charge.as_ref().map(|(_, e)| read_threshold(e)),
        tlp_profile: tlp_profile().await,
        protection: protection_info(),
    }
}

#[tauri::command]
pub async fn set_brightness(percent: i64) -> Result<(), String> {
    let Some(device) = brightness_device().await else {
        return Err("No controllable backlight found".into());
    };
    // Clamp before handing the value to brightnessctl.
    let pct = format!("{}%", percent.clamp(0, 100));
    Command::new("brightnessctl")
        .args(["--device", &device, "set", &pct])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn charge_threshold_write(which: &str, percent: i64) -> Result<(String, i64), String> {
    if which != "start" && which != "end" {
        return Err("threshold must be start or end".into());
    }
    Ok((which.to_string(), percent.clamp(0, 100)))
}

#[tauri::command]
pub async fn set_charge_threshold(which: String, percent: i64) -> Result<(), String> {
    let (which, percent) = charge_threshold_write(&which, percent)?;
    let Some((start, end)) = charge_threshold_paths() else {
        return Err("No charge threshold support on this hardware".into());
    };
    let path = if which == "start" { start } else { end };
    if !valid_protect_path(&path) {
        return Err("Refusing to write to an unexpected battery path.".into());
    }
    // GNU tee writes stdin to its path operands; the percent must be piped,
    // not passed as a second path argument.
    let input = format!("{percent}\n");
    if util::run_with_stdin(&["pkexec", "tee", &path.display().to_string()], &input).await {
        Ok(())
    } else {
        Err("Failed to set charge threshold".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_threshold_clamps_and_restricts_which() {
        assert_eq!(
            charge_threshold_write("start", 80).unwrap(),
            ("start".into(), 80)
        );
        assert_eq!(
            charge_threshold_write("end", 150).unwrap(),
            ("end".into(), 100)
        );
        assert_eq!(
            charge_threshold_write("start", -5).unwrap(),
            ("start".into(), 0)
        );
        assert!(charge_threshold_write("both", 50).is_err());
        assert!(charge_threshold_write("-start", 50).is_err());
    }

    use std::path::{Path, PathBuf};

    fn fake_sysfs(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("bos-settings-power-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let ps = root.join("power_supply");
        let ideapad = root.join("ideapad_acpi");
        std::fs::create_dir_all(ps.join("BAT1")).unwrap();
        std::fs::create_dir_all(&ideapad).unwrap();
        (root, ps, ideapad)
    }

    #[test]
    fn charge_types_parsing_finds_active_and_supported() {
        let (active, all) = parse_charge_types("[Standard] Long_Life\n");
        assert_eq!(active.as_deref(), Some("Standard"));
        assert_eq!(all, vec!["Standard", "Long_Life"]);
        let (active, _) = parse_charge_types("Standard [Long_Life]");
        assert_eq!(active.as_deref(), Some("Long_Life"));
        assert_eq!(parse_charge_types("").0, None);
    }

    #[test]
    fn detection_prefers_thresholds_then_charge_types_then_conservation() {
        let (root, ps, ideapad) = fake_sysfs("detect");
        std::fs::create_dir_all(ideapad.join("VPC2004:00")).unwrap();
        std::fs::write(ideapad.join("VPC2004:00/conservation_mode"), "0\n").unwrap();
        assert!(matches!(
            detect_protection_in(&ps, &ideapad),
            Some(Protection::Conservation(_))
        ));
        std::fs::write(ps.join("BAT1/charge_types"), "[Standard] Long_Life\n").unwrap();
        assert!(matches!(
            detect_protection_in(&ps, &ideapad),
            Some(Protection::ChargeTypes(_))
        ));
        std::fs::write(ps.join("BAT1/charge_control_end_threshold"), "100\n").unwrap();
        assert!(matches!(
            detect_protection_in(&ps, &ideapad),
            Some(Protection::Thresholds { .. })
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn charge_types_without_long_life_is_not_protection() {
        let (root, ps, ideapad) = fake_sysfs("nolife");
        std::fs::write(ps.join("BAT1/charge_types"), "[Fast] Standard\n").unwrap();
        assert_eq!(detect_protection_in(&ps, &ideapad), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn nothing_detected_means_no_switch() {
        let (root, ps, ideapad) = fake_sysfs("none");
        assert_eq!(detect_protection_in(&ps, &ideapad), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn threshold_writes_never_leave_start_above_end() {
        let (root, ps, _) = fake_sysfs("order");
        let start = ps.join("BAT1/charge_control_start_threshold");
        let end = ps.join("BAT1/charge_control_end_threshold");
        std::fs::write(&start, "0\n").unwrap();
        std::fs::write(&end, "100\n").unwrap();
        let p = Protection::Thresholds {
            start: start.clone(),
            end: end.clone(),
        };
        let on = protect_writes(&p, true);
        assert_eq!(
            on,
            vec![(end.clone(), "80\n".into()), (start.clone(), "75\n".into())]
        );
        let off = protect_writes(&p, false);
        assert_eq!(off, vec![(start, "0\n".into()), (end, "100\n".into())]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fixed_mode_writes_are_exact_values() {
        let ct =
            Protection::ChargeTypes(PathBuf::from("/sys/class/power_supply/BAT1/charge_types"));
        assert_eq!(protect_writes(&ct, true)[0].1, "Long_Life\n");
        assert_eq!(protect_writes(&ct, false)[0].1, "Standard\n");
        let cm = Protection::Conservation(PathBuf::from(
            "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00/conservation_mode",
        ));
        assert_eq!(protect_writes(&cm, true)[0].1, "1\n");
        assert_eq!(protect_writes(&cm, false)[0].1, "0\n");
    }

    #[test]
    fn only_known_sysfs_files_may_be_written() {
        assert!(valid_protect_path(Path::new(
            "/sys/class/power_supply/BAT1/charge_types"
        )));
        assert!(valid_protect_path(Path::new(
            "/sys/class/power_supply/BAT0/charge_control_end_threshold"
        )));
        assert!(valid_protect_path(Path::new(
            "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00/conservation_mode"
        )));
        // wrong directory for the file name, traversal, other files, whitespace
        assert!(!valid_protect_path(Path::new(
            "/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00/charge_types"
        )));
        assert!(!valid_protect_path(Path::new(
            "/sys/class/power_supply/../../etc/passwd"
        )));
        assert!(!valid_protect_path(Path::new(
            "/sys/class/power_supply/BAT1/status"
        )));
        assert!(!valid_protect_path(Path::new("/etc/shadow")));
        assert!(!valid_protect_path(Path::new(
            "/sys/class/power_supply/BAT1/charge_types x"
        )));
        assert!(!valid_protect_path(Path::new("charge_types")));
    }
}
