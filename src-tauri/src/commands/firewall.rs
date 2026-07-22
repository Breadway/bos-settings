//! ufw firewall rules. `ufw status` itself requires root (confirmed against
//! the installed ufw script), so unlike every other read-only panel, this
//! doesn't query eagerly — the frontend only calls `get_firewall_status`
//! when the user clicks Refresh, deferring the one unavoidable polkit
//! prompt to an explicit action instead of forcing it on app open.

use serde::Serialize;
use tokio::process::Command;

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
                Some(127) => "no polkit authentication agent is available in this session".to_string(),
                Some(code) => format!("pkexec exited with status {code}"),
                None => "pkexec was terminated by a signal".to_string(),
            }
        } else {
            stderr
        });
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let active = text.lines().next().is_some_and(|l| l.trim() == "Status: active");
    let rules = text
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if !l.starts_with('[') {
                return None;
            }
            let (num, rest) = l.split_once(']')?;
            let number = num.trim_start_matches('[').trim().to_string();
            Some(FirewallRule { number, text: rest.trim().to_string() })
        })
        .collect();
    Ok(FirewallStatus { active, rules })
}

#[tauri::command]
pub async fn set_firewall_enabled(enabled: bool) -> Result<(), String> {
    let verb = if enabled { "enable" } else { "disable" };
    let output = Command::new("pkexec").args(["ufw", "--force", verb]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub async fn add_firewall_rule(rule: String) -> Result<(), String> {
    let output = Command::new("pkexec").args(["ufw", "allow", rule.trim()]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[tauri::command]
pub async fn remove_firewall_rule(number: String) -> Result<(), String> {
    let output = Command::new("pkexec").args(["ufw", "--force", "delete", &number]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
