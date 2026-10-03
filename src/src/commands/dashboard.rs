use super::util;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
#[derive(Clone, Serialize, Deserialize)]
pub struct RadioState {
    pub id: u32,
    pub kind: String,
    pub blocked: bool,
    pub hardware_blocked: bool,
}
#[tauri::command]
pub async fn get_radio_states() -> Result<Vec<RadioState>, String> {
    let out = Command::new("rfkill")
        .args(["--json"])
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(util::fail_output(&out, "Reading radio state"));
    }
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let rows = value
        .get("rfkilldevices")
        .and_then(|v| v.as_array())
        .ok_or("Invalid radio status")?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let kind = r.get("type")?.as_str()?;
            if !["wlan", "bluetooth", "wwan"].contains(&kind) {
                return None;
            }
            let id = u32::try_from(r.get("id")?.as_u64()?).ok()?;
            Some(RadioState {
                id,
                kind: kind.into(),
                blocked: r.get("soft").and_then(|v| v.as_str()) == Some("blocked"),
                hardware_blocked: r.get("hard").and_then(|v| v.as_str()) == Some("blocked"),
            })
        })
        .collect())
}
async fn block_radios(blocked: bool, ids: &[u32]) -> Result<(), String> {
    for id in ids {
        let out = Command::new("pkexec")
            .args([
                "rfkill",
                if blocked { "block" } else { "unblock" },
                &id.to_string(),
            ])
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(util::fail_output(&out, "Changing airplane mode"));
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn set_airplane_mode(enabled: bool) -> Result<Vec<RadioState>, String> {
    let previous = get_radio_states().await?;
    let ids = previous.iter().map(|r| r.id).collect::<Vec<_>>();
    if ids.is_empty() {
        return Err("No wireless radios found".into());
    }
    block_radios(enabled, &ids).await?;
    get_radio_states().await
}
#[tauri::command]
pub async fn restore_radio_states(radios: Vec<RadioState>) -> Result<(), String> {
    let current = get_radio_states().await?;
    for r in radios {
        if current.iter().any(|c| c.id == r.id && c.kind == r.kind) {
            block_radios(r.blocked, &[r.id]).await?
        }
    }
    Ok(())
}
#[derive(Serialize)]
pub struct Account {
    username: String,
    display_name: String,
}
#[tauri::command]
pub fn get_current_account() -> Account {
    let username = std::env::var("USER").unwrap_or_else(|_| "User".into());
    let display_name = std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                let f = l.split(':').collect::<Vec<_>>();
                (f.len() > 4 && f[0] == username)
                    .then(|| f[4].split(',').next().unwrap_or("").to_string())
            })
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| username.clone());
    Account {
        username,
        display_name,
    }
}
