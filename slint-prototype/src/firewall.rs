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

fn valid_firewall_rule(rule: &str) -> bool {
    let rule = rule.trim().replace(' ', "/");
    let (ports, protocol) = match rule.split_once('/') {
        Some((ports, protocol)) => (ports, Some(protocol)),
        None => (rule.as_str(), None),
    };
    if let Some(protocol) = protocol {
        if !matches!(protocol, "tcp" | "udp") {
            return false;
        }
    }
    let port_ok = |text: &str| {
        text.parse::<u32>()
            .map(|port| (1..=65535).contains(&port))
            .unwrap_or(false)
    };
    match ports.split_once(':') {
        Some((start, end)) => {
            protocol.is_some()
                && port_ok(start)
                && port_ok(end)
                && start.parse::<u32>().unwrap_or(0) <= end.parse::<u32>().unwrap_or(0)
        }
        None => port_ok(ports),
    }
}

fn valid_rule_number(number: &str) -> bool {
    crate::util::valid_number_id(number.trim())
}

fn ufw(args: &[&str]) -> Result<(), String> {
    let out = Command::new("pkexec")
        .arg("ufw")
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn add_rule(rule: &str) -> Result<(), String> {
    let rule = rule.trim();
    if !valid_firewall_rule(rule) {
        return Err("invalid firewall rule".into());
    }
    ufw(&["allow", rule])
}

pub fn remove_allow(rule: &str) -> Result<(), String> {
    let rule = rule.trim();
    if !valid_firewall_rule(rule) {
        return Err("invalid firewall rule".into());
    }
    ufw(&["--force", "delete", "allow", rule])
}

pub fn remove_rule(number: &str) -> Result<(), String> {
    if !valid_rule_number(number) {
        return Err("invalid rule number".into());
    }
    ufw(&["--force", "delete", number.trim()])
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
