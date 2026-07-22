use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct InstalledPackage {
    name: String,
    version: String,
}

#[tauri::command]
pub fn get_installed_packages() -> Vec<InstalledPackage> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let path = std::path::Path::new(&home).join(".local/state/bakery/installed.json");

    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    // installed.json is {"packages": {name: {version, binaries, services}}},
    // not a flat map of package name to metadata.
    let Some(packages) = parsed.get_mut("packages").map(std::mem::take) else {
        return Vec::new();
    };
    let Ok(packages) = serde_json::from_value::<HashMap<String, serde_json::Value>>(packages) else {
        return Vec::new();
    };

    let mut list: Vec<InstalledPackage> = packages
        .into_iter()
        .map(|(name, val)| {
            let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            InstalledPackage { name, version }
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}
