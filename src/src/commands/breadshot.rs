//! Screenshots (breadshot). Binds are read-only here — edit them on the
//! Keybinds panel. Config lives at `~/.config/breadshot/config.toml` and
//! matches breadshot's own `Config` (every field optional).

use serde::{Deserialize, Serialize};

use super::config;
use super::keybinds;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadshot/config.toml")
}

#[derive(Serialize, Deserialize)]
pub struct BreadshotConfig {
    save_dir: String,
    silent: bool,
    freeze: bool,
    notif_timeout: i64,
    date_format: String,
}

impl Default for BreadshotConfig {
    fn default() -> Self {
        Self {
            save_dir: "~/Pictures/Screenshots".into(),
            silent: false,
            freeze: false,
            notif_timeout: 5000,
            date_format: "%Y-%m-%d-%H%M%S".into(),
        }
    }
}

#[derive(Serialize)]
pub struct ShotBind {
    shortcut: String,
    command: String,
}

#[tauri::command]
pub fn get_breadshot_config() -> BreadshotConfig {
    let doc = config::load_doc(&config_path());
    let mut cfg = BreadshotConfig::default();
    if let Some(dir) = config::get_str(&doc, &["save_dir"]) {
        cfg.save_dir = dir;
    }
    if let Some(v) = config::get_bool(&doc, &["silent"]) {
        cfg.silent = v;
    }
    if let Some(v) = config::get_bool(&doc, &["freeze"]) {
        cfg.freeze = v;
    }
    if let Some(v) = config::get_i64(&doc, &["notif_timeout"]) {
        cfg.notif_timeout = v;
    }
    if let Some(v) = config::get_str(&doc, &["date_format"]) {
        cfg.date_format = v;
    }
    cfg
}

#[tauri::command]
pub fn save_breadshot_config(cfg: BreadshotConfig) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    config::set_str(&mut doc, &["save_dir"], &cfg.save_dir);
    config::set_bool(&mut doc, &["silent"], cfg.silent);
    config::set_bool(&mut doc, &["freeze"], cfg.freeze);
    config::set_i64(&mut doc, &["notif_timeout"], cfg.notif_timeout.max(0));
    config::set_str(&mut doc, &["date_format"], &cfg.date_format);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}

/// Documented BOS defaults (SUPER+Shift+S/C/P) used when binds.json has no
/// breadshot exec entries — still accurate as a cheatsheet even on a
/// machine whose binds were rewritten.
fn documented_defaults() -> Vec<ShotBind> {
    vec![
        ShotBind {
            shortcut: "Super+Shift+S".into(),
            command: "breadshot region".into(),
        },
        ShotBind {
            shortcut: "Super+Shift+C".into(),
            command: "breadshot region --clipboard-only".into(),
        },
        ShotBind {
            shortcut: "Super+Shift+P".into(),
            command: "breadshot active-output".into(),
        },
    ]
}

fn format_shortcut(mods: Option<&[String]>, key: Option<&str>, default_mods: &[String]) -> String {
    let mods = mods.unwrap_or(default_mods);
    let mut parts: Vec<String> = mods
        .iter()
        .map(|m| match m.to_ascii_uppercase().as_str() {
            "SUPER" | "MOD4" => "Super".into(),
            "SHIFT" => "Shift".into(),
            "CTRL" | "CONTROL" => "Ctrl".into(),
            "ALT" | "MOD1" => "Alt".into(),
            other => other.to_string(),
        })
        .collect();
    if let Some(k) = key {
        if !k.is_empty() {
            parts.push(k.to_string());
        }
    }
    parts.join("+")
}

/// Read-only: breadshot exec binds from binds.json, or the documented
/// Super+Shift+S/C/P cheatsheet if none are defined.
#[tauri::command]
pub fn get_breadshot_binds() -> Vec<ShotBind> {
    let from_file = keybinds::breadshot_binds();
    if from_file.is_empty() {
        documented_defaults()
    } else {
        from_file
            .into_iter()
            .map(|b| ShotBind {
                shortcut: format_shortcut(b.mods.as_deref(), b.key.as_deref(), &b.default_mods),
                command: b.command,
            })
            .collect()
    }
}

/// Interactive region capture, clipboard only — no file written.
#[tauri::command]
pub fn breadshot_region_clipboard() {
    let _ = std::process::Command::new("breadshot")
        .args(["region", "--clipboard-only"])
        .spawn();
}
