//! Timezone + NTP, over `timedatectl`. `systemd-timesyncd` is enabled by
//! default, so NTP sync is on out of the box — this is mostly for picking a
//! timezone and confirming sync is healthy.

use serde::Serialize;
use tokio::process::Command;

async fn show_property(prop: &str) -> String {
    Command::new("timedatectl")
        .args(["show", &format!("--property={prop}")])
        .output()
        .await
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .strip_prefix(&format!("{prop}="))
                .map(str::to_string)
        })
        .unwrap_or_default()
}

async fn list_timezones() -> Vec<String> {
    Command::new("timedatectl")
        .arg("list-timezones")
        .output()
        .await
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

async fn current_time_label() -> String {
    Command::new("date")
        .arg("+%A, %d %B %Y  %H:%M")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[derive(Serialize)]
pub struct DateTimeInfo {
    current_time: String,
    timezones: Vec<String>,
    current_tz: String,
    ntp_enabled: bool,
    ntp_synced: bool,
}

#[tauri::command]
pub async fn get_datetime_info() -> DateTimeInfo {
    DateTimeInfo {
        current_time: current_time_label().await,
        timezones: list_timezones().await,
        current_tz: show_property("Timezone").await,
        ntp_enabled: show_property("NTP").await == "yes",
        ntp_synced: show_property("NTPSynchronized").await == "yes",
    }
}

/// Reject flags, path traversal, and newlines before we ever exec. Charset
/// matches IANA names (`Area/City`, `UTC`, `Etc/GMT+6`).
fn timezone_looks_safe(tz: &str) -> bool {
    let tz = tz.trim();
    if tz.is_empty() || tz.len() > 64 || tz.starts_with('-') {
        return false;
    }
    if tz.contains('\n') || tz.contains('\r') || tz.contains('\0') || tz.contains("..") {
        return false;
    }
    tz.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-'))
}

#[tauri::command]
pub async fn set_timezone(tz: String) -> Result<(), String> {
    let tz = tz.trim();
    if !timezone_looks_safe(tz) {
        return Err("invalid timezone".into());
    }
    let listed = list_timezones().await;
    if !listed.is_empty() && !listed.iter().any(|t| t == tz) {
        return Err("unknown timezone".into());
    }
    let output = Command::new("pkexec")
        .args(["timedatectl", "set-timezone", tz])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Error — check the timezone name".into())
    }
}

#[tauri::command]
pub async fn set_ntp_enabled(enabled: bool) -> Result<(), String> {
    let val = if enabled { "true" } else { "false" };
    Command::new("pkexec")
        .args(["timedatectl", "set-ntp", val])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timezone_rejects_flags_and_traversal() {
        assert!(timezone_looks_safe("UTC"));
        assert!(timezone_looks_safe("America/New_York"));
        assert!(timezone_looks_safe("Etc/GMT+6"));
        assert!(!timezone_looks_safe(""));
        assert!(!timezone_looks_safe("-UTC"));
        assert!(!timezone_looks_safe("--help"));
        assert!(!timezone_looks_safe("America/../UTC"));
        assert!(!timezone_looks_safe("UTC\n--adjust"));
        assert!(!timezone_looks_safe("UTC;reboot"));
    }
}
