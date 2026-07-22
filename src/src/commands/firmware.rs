use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FwDevice {
    name: String,
    version: String,
}

#[tauri::command]
pub async fn get_updatable_firmware() -> Vec<FwDevice> {
    let Ok(output) = tokio::process::Command::new("fwupdmgr").args(["get-devices", "--json"]).output().await else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return Vec::new();
    };
    let Some(devices) = root.get("Devices").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|d| {
            d.get("Flags").and_then(|f| f.as_array()).is_some_and(|flags| flags.iter().any(|f| f.as_str() == Some("updatable")))
        })
        .filter_map(|d| {
            Some(FwDevice {
                name: d.get("Name")?.as_str()?.to_string(),
                version: d.get("Version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            })
        })
        .collect()
}
