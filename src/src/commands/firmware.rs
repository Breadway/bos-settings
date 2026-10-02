use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FwDevice {
    name: String,
    version: String,
    security_database: bool,
}

#[tauri::command]
pub async fn get_updatable_firmware() -> Vec<FwDevice> {
    let Ok(output) = tokio::process::Command::new("fwupdmgr")
        .args(["get-devices", "--json"])
        .output()
        .await
    else {
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
            d.get("Flags")
                .and_then(|f| f.as_array())
                .is_some_and(|flags| flags.iter().any(|f| f.as_str() == Some("updatable")))
        })
        .filter_map(|d| {
            Some(FwDevice {
                name: d.get("Name")?.as_str()?.to_string(),
                security_database: is_security_database(d),
                version: d
                    .get("Version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            })
        })
        .collect()
}

fn is_security_database(device: &serde_json::Value) -> bool {
    let name = device
        .get("Name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("uefi dbx")
        || name.contains("uefi ca")
        || name.contains("kek ca")
        || device
            .get("Plugin")
            .and_then(|n| n.as_str())
            .is_some_and(|p| ["uefi-dbx", "uefi-kek", "uefi-db", "uefi-pk"].contains(&p))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separate_secure_boot_databases_from_hardware() {
        assert!(is_security_database(
            &serde_json::json!({"Name":"UEFI dbx"})
        ));
        assert!(is_security_database(
            &serde_json::json!({"Name":"Microsoft UEFI CA"})
        ));
        assert!(is_security_database(
            &serde_json::json!({"Name":"Certificates", "Plugin":"uefi-kek"})
        ));
        assert!(!is_security_database(
            &serde_json::json!({"Name":"System Firmware", "Plugin":"uefi_capsule"})
        ));
    }
}
