//! ufw firewall state. Status reads need no privilege; enabling and
//! disabling go through systemd, which asks polkit for authorization.

use std::process::Command;

pub fn service_active() -> Option<bool> {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "ufw.service"])
        .output()
        .ok()
        .map(|out| out.status.success())
}

pub struct Status {
    pub active: bool,
    pub rules: Vec<(String, String)>,
}

pub fn status() -> Result<Status, String> {
    let out = Command::new("ufw")
        .args(["status", "numbered"])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "ufw needs more privilege to list rules".into()
        } else {
            stderr
        });
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let active = text
        .lines()
        .next()
        .is_some_and(|line| line.trim() == "Status: active");
    let rules = text
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix('[')?;
            let (number, text) = rest.split_once(']')?;
            Some((number.trim().to_string(), text.trim().to_string()))
        })
        .collect();
    Ok(Status { active, rules })
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let verb = if enabled { "start" } else { "stop" };
    let out = Command::new("systemctl")
        .args([verb, "ufw.service"])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "the firewall service refused the change".into()
        } else {
            stderr
        })
    }
}
