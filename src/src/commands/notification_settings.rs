//! The notification server owns notification settings, exposed through a fixed D-Bus API.
use super::util;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tokio::process::Command;
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppPolicy {
    All,
    Banners,
    History,
    Off,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub dnd: bool,
    pub schedule: bool,
    pub start: String,
    pub end: String,
    pub lock_screen: bool,
    pub sound: bool,
    pub apps: BTreeMap<String, AppPolicy>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}
async fn call(
    method: &str,
    prefs: Option<&NotificationPreferences>,
) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new("busctl");
    cmd.args([
        "--user",
        "--json=short",
        "call",
        "org.freedesktop.Notifications",
        "/dev/breadway/Bar",
        "dev.breadway.Bar",
        method,
    ]);
    if let Some(p) = prefs {
        cmd.arg("s")
            .arg(serde_json::to_string(p).map_err(|e| e.to_string())?);
    }
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "The running notification server needs the notification settings API. {}",
            util::fail_output(&out, "Notification settings")
        ));
    }
    if prefs.is_some() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn get_notification_preferences() -> Result<NotificationPreferences, String> {
    let result = call("GetNotificationPreferences", None).await?;
    let json = result
        .get("data")
        .and_then(|v| v.get(0))
        .and_then(|v| v.as_str())
        .ok_or("Invalid notification settings response")?;
    serde_json::from_str(json).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn save_notification_preferences(
    preferences: NotificationPreferences,
) -> Result<(), String> {
    call("SetNotificationPreferences", Some(&preferences)).await?;
    Ok(())
}
