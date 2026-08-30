//! ufw firewall rules. `ufw status` itself requires root (confirmed against
//! the installed ufw script), so unlike every other read-only panel, this
//! doesn't query eagerly — the frontend only calls `get_firewall_status`
//! when the user clicks Refresh, deferring the one unavoidable polkit
//! prompt to an explicit action instead of forcing it on app open.

use serde::Serialize;
use tokio::process::Command;

use super::util;

#[derive(Serialize, Clone)]
pub struct FirewallRule {
    number: String,
    text: String,
}

#[derive(Serialize)]
pub struct FirewallStatus {
    active: bool,
    rules: Vec<FirewallRule>,
}

/// One `pkexec ufw status numbered` call, parsed for both the active/
/// inactive line and the numbered rules.
#[tauri::command]
pub async fn get_firewall_status() -> Result<FirewallStatus, String> {
    let output = Command::new("pkexec")
        .args(["ufw", "status", "numbered"])
        .output()
        .await
        .map_err(|e| format!("couldn't run pkexec: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            match output.status.code() {
                Some(127) => {
                    "no polkit authentication agent is available in this session".to_string()
                }
                Some(code) => format!("pkexec exited with status {code}"),
                None => "pkexec was terminated by a signal".to_string(),
            }
        } else {
            stderr
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let active = text
        .lines()
        .next()
        .is_some_and(|l| l.trim() == "Status: active");
    let rules = text
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if !l.starts_with('[') {
                return None;
            }
            let (num, rest) = l.split_once(']')?;
            let number = num.trim_start_matches('[').trim().to_string();
            Some(FirewallRule {
                number,
                text: rest.trim().to_string(),
            })
        })
        .collect();
    Ok(FirewallStatus { active, rules })
}

#[tauri::command]
pub async fn set_firewall_enabled(enabled: bool) -> Result<(), String> {
    let verb = if enabled { "enable" } else { "disable" };
    let output = Command::new("pkexec")
        .args(["ufw", "--force", verb])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Port, optional `/tcp`/`/udp`, or optional space-separated proto. No
/// service names, IPs, or flags — those become extra `ufw allow` operands.
fn valid_firewall_rule(rule: &str) -> bool {
    let rule = rule.trim();
    if rule.is_empty() || rule.len() > 16 || rule.starts_with('-') {
        return false;
    }
    if rule.contains('\n') || rule.contains('\r') || rule.contains('\0') {
        return false;
    }
    let (port, proto) = if let Some((p, rest)) = rule.split_once('/') {
        (p, Some(rest))
    } else if let Some((p, rest)) = rule.split_once(' ') {
        (p, Some(rest.trim()))
    } else {
        (rule, None)
    };
    let Ok(n) = port.parse::<u16>() else {
        return false;
    };
    if n == 0 {
        return false;
    }
    match proto {
        None => true,
        Some(p) => p == "tcp" || p == "udp",
    }
}

fn valid_rule_number(number: &str) -> bool {
    let t = number.trim();
    !t.is_empty()
        && t.len() <= 8
        && t.bytes().all(|b| b.is_ascii_digit())
        && t.parse::<u32>().is_ok_and(|n| n > 0)
}

#[tauri::command]
pub async fn add_firewall_rule(rule: String) -> Result<(), String> {
    let rule = rule.trim();
    if !valid_firewall_rule(rule) {
        return Err("invalid firewall rule".into());
    }
    let output = Command::new("pkexec")
        .args(["ufw", "allow", rule])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub async fn remove_firewall_rule(number: String) -> Result<(), String> {
    if !valid_rule_number(&number) {
        return Err("invalid rule number".into());
    }
    let output = Command::new("pkexec")
        .args(["ufw", "--force", "delete", number.trim()])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firewall_rule_is_port_and_optional_proto() {
        assert!(valid_firewall_rule("22"));
        assert!(valid_firewall_rule("8080/tcp"));
        assert!(valid_firewall_rule("53/udp"));
        assert!(valid_firewall_rule("80 tcp"));
        assert!(!valid_firewall_rule("OpenSSH"));
        assert!(!valid_firewall_rule("-f"));
        assert!(!valid_firewall_rule("22;id"));
        assert!(!valid_firewall_rule("22/tcp\nallow 23"));
        assert!(!valid_firewall_rule("0"));
        assert!(!valid_firewall_rule("65536"));
        assert!(!valid_firewall_rule("22/all"));
    }

    #[test]
    fn firewall_delete_is_positive_int() {
        assert!(valid_rule_number("1"));
        assert!(valid_rule_number("12"));
        assert!(!valid_rule_number("0"));
        assert!(!valid_rule_number("-1"));
        assert!(!valid_rule_number("1;2"));
        assert!(!valid_rule_number("1\n2"));
        assert!(!valid_rule_number(""));
    }
}
