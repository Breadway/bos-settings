//! Live systemd `--user` unit status, start/stop/restart and logs.

use std::process::Command;

pub struct Status {
    pub active: bool,
    pub enabled: bool,
}

/// Units the settings pages are allowed to touch.
pub const ALLOWED_UNITS: &[&str] = &[
    "breadd.service",
    "breadclipd.service",
    "breadcrumbs.service",
    "breadmill.service",
    "breadbox-sync.service",
];

fn allowed(unit: &str) -> bool {
    ALLOWED_UNITS.contains(&unit)
}

pub fn status(unit: &str) -> Result<Status, String> {
    if !allowed(unit) {
        return Err("unknown service".into());
    }
    let active = Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    let enabled = Command::new("systemctl")
        .args(["--user", "is-enabled", "--quiet", unit])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    Ok(Status { active, enabled })
}

pub fn action(unit: &str, verb: &str) -> Result<(), String> {
    if !allowed(unit) {
        return Err("unknown service".into());
    }
    if !["start", "stop", "restart"].contains(&verb) {
        return Err("unknown action".into());
    }
    let out = Command::new("systemctl")
        .args(["--user", verb, unit])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn open_logs(unit: &str) -> Result<(), String> {
    if !allowed(unit) {
        return Err("unknown service".into());
    }
    Command::new("kitty")
        .args(["-e", "journalctl", "--user", "-u", unit, "-f"])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_units_are_allowed() {
        assert!(allowed("breadd.service"));
        assert!(!allowed("sshd.service"));
        assert!(status("unknown.service").is_err());
    }
}
