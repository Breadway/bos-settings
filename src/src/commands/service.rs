//! Live systemd `--user` unit status plus start/stop/restart/logs — every
//! bread-ecosystem panel whose app is actually a daemon (not just a config
//! file) gets this. Ported from `src/ui/widgets.rs`'s `service_control`;
//! the `critical`-unit confirm-before-stop behavior moves to the frontend
//! (a confirm step gating the call to `service_action`), since that's UI
//! policy, not something the command itself needs to know.

use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Serialize)]
pub struct ServiceStatus {
    active: bool,
    enabled: bool,
}

#[derive(Deserialize)]
pub enum ServiceAction {
    Start,
    Stop,
    Restart,
}

async fn systemctl_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn systemctl_enabled(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-enabled", "--quiet", unit])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn get_service_status(unit: String) -> ServiceStatus {
    ServiceStatus {
        active: systemctl_active(&unit).await,
        enabled: systemctl_enabled(&unit).await,
    }
}

#[tauri::command]
pub async fn service_action(unit: String, action: ServiceAction) -> Result<(), String> {
    let verb = match action {
        ServiceAction::Start => "start",
        ServiceAction::Stop => "stop",
        ServiceAction::Restart => "restart",
    };
    let output = Command::new("systemctl")
        .args(["--user", verb, &unit])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Opens a terminal following the unit's journal — same as today's GTK
/// panel, no reason to pull an open-ended `journalctl -f` tail into the
/// webview.
#[tauri::command]
pub fn open_logs(unit: String) {
    let _ = std::process::Command::new("kitty")
        .args(["-e", "journalctl", "--user", "-u", &unit, "-f"])
        .spawn();
}
