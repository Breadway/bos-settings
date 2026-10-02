//! Sharing controls with fixed commands and no webview-supplied argv.
use super::util;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Serialize)]
pub struct SharingStatus {
    hostname: String,
    ssh_installed: bool,
    ssh_running: bool,
    localsend_installed: bool,
    phone_link_installed: bool,
}
async fn flatpak_localsend() -> bool {
    Command::new("flatpak")
        .args(["info", "org.localsend.localsend_app"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
#[tauri::command]
pub async fn get_sharing_status() -> SharingStatus {
    let ssh_running = Command::new("systemctl")
        .args(["is-active", "sshd.service"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    SharingStatus {
        hostname: std::fs::read_to_string("/etc/hostname")
            .unwrap_or_else(|_| "Unknown".into())
            .trim()
            .into(),
        ssh_installed: util::command_exists("sshd"),
        ssh_running,
        localsend_installed: util::command_exists("localsend") || flatpak_localsend().await,
        phone_link_installed: util::command_exists("kdeconnect-settings"),
    }
}
#[tauri::command]
pub async fn set_remote_login(enabled: bool) -> Result<(), String> {
    let operation = if enabled { "enable" } else { "disable" };
    let out = Command::new("pkexec")
        .args(["systemctl", operation, "--now", "sshd.service"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Changing remote login"))
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharingApp {
    LocalSend,
    PhoneLink,
}
#[tauri::command]
pub async fn open_sharing_app(app: SharingApp) -> Result<(), String> {
    let mut command = match app {
        SharingApp::LocalSend if util::command_exists("localsend") => {
            std::process::Command::new("localsend")
        }
        SharingApp::LocalSend if flatpak_localsend().await => {
            let mut cmd = std::process::Command::new("flatpak");
            cmd.args(["run", "org.localsend.localsend_app"]);
            cmd
        }
        SharingApp::PhoneLink if util::command_exists("kdeconnect-settings") => {
            std::process::Command::new("kdeconnect-settings")
        }
        _ => return Err("Install the sharing app first".into()),
    };
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn open_notification_history() -> Result<(), String> {
    let out = Command::new("breadbar")
        .arg("--history")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Opening notification history"))
    }
}
