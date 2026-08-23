//! Battery/power status (upower), brightness (brightnessctl), and TLP's
//! current profile. Deliberately no AC/Battery/Performance *switcher* — TLP
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
/// actually exposes them — genuinely hardware-dependent, not every install
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

#[derive(Serialize)]
pub struct PowerInfo {
    battery: Vec<(String, String)>,
    power_source: String,
    brightness_pct: Option<u32>,
    charge_start: Option<i64>,
    charge_end: Option<i64>,
    tlp_profile: Option<String>,
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
    }
}

#[tauri::command]
pub async fn set_brightness(percent: i64) -> Result<(), String> {
    let Some(device) = brightness_device().await else {
        return Err("No controllable backlight found".into());
    };
    let pct = format!("{percent}%");
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
    // GNU tee writes stdin to its path operands — the percent must be piped,
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
}
