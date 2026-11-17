//! breadshot/config.toml: where screenshots go and whether they notify.

use std::path::PathBuf;
use std::process::Command;

use crate::config;

pub struct Config {
    pub save_dir: String,
    pub silent: bool,
    pub freeze: bool,
}

pub fn path() -> PathBuf {
    config::config_dir().join("breadshot/config.toml")
}

pub fn load() -> Config {
    let doc = config::load_doc(&path());
    Config {
        save_dir: config::get_str(&doc, &["save_dir"])
            .unwrap_or_else(|| "~/Pictures/Screenshots".into()),
        silent: config::get_bool(&doc, &["silent"]).unwrap_or(false),
        freeze: config::get_bool(&doc, &["freeze"]).unwrap_or(false),
    }
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let dir = cfg.save_dir.trim();
    if dir.is_empty() || dir.contains('\n') || dir.starts_with('-') {
        return Err("use a folder path".into());
    }
    let path = path();
    let mut doc = config::load_doc(&path);
    config::set_str(&mut doc, &["save_dir"], dir);
    config::set_bool(&mut doc, &["silent"], cfg.silent);
    config::set_bool(&mut doc, &["freeze"], cfg.freeze);
    config::save_doc(&path, &doc)
}

/// breadshot exec binds from binds.json, or the documented defaults when no
/// binds are defined.
pub fn binds() -> Vec<(String, String)> {
    let path = config::config_dir().join("hypr/binds.json");
    let mut found = Vec::new();
    if let Ok(text) = std::fs::read_to_string(path) {
        if let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) {
            let default_mods: Vec<String> = root
                .get("default_mods")
                .and_then(|value| value.as_array())
                .map(|mods| {
                    mods.iter()
                        .filter_map(|entry| entry.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let mut lists: Vec<&Vec<serde_json::Value>> = Vec::new();
            for key in ["bindings", "globals", "common"] {
                if let Some(list) = root.get(key).and_then(|value| value.as_array()) {
                    lists.push(list);
                }
            }
            if let Some(layouts) = root.get("layouts").and_then(|value| value.as_object()) {
                for list in layouts.values().filter_map(|value| value.as_array()) {
                    lists.push(list);
                }
            }
            for list in lists {
                for bind in list {
                    if bind.get("action").and_then(|value| value.as_str()) != Some("exec") {
                        continue;
                    }
                    let Some(command) = bind.get("command").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    if command
                        .split_whitespace()
                        .next()
                        .is_none_or(|bin| bin != "breadshot" && !bin.ends_with("/breadshot"))
                    {
                        continue;
                    }
                    let mods: Vec<String> = bind
                        .get("mods")
                        .and_then(|value| value.as_array())
                        .map(|mods| {
                            mods.iter()
                                .filter_map(|entry| entry.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_else(|| default_mods.clone());
                    let mut parts: Vec<String> = mods
                        .iter()
                        .map(|modifier| match modifier.to_ascii_uppercase().as_str() {
                            "SUPER" | "MOD4" => "Super".into(),
                            "SHIFT" => "Shift".into(),
                            "CTRL" | "CONTROL" => "Ctrl".into(),
                            "ALT" | "MOD1" => "Alt".into(),
                            other => other.to_string(),
                        })
                        .collect();
                    if let Some(key) = bind.get("key").and_then(|value| value.as_str()) {
                        if !key.is_empty() {
                            parts.push(key.to_string());
                        }
                    }
                    found.push((parts.join("+"), command.to_string()));
                }
            }
        }
    }
    if found.is_empty() {
        vec![
            ("Super+Shift+S".into(), "breadshot region".into()),
            (
                "Super+Shift+C".into(),
                "breadshot region --clipboard-only".into(),
            ),
            ("Super+Shift+P".into(), "breadshot active-output".into()),
        ]
    } else {
        found
    }
}

pub fn region_to_clipboard() -> Result<(), String> {
    Command::new("breadshot")
        .args(["region", "--clipboard-only"])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
