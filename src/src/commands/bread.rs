//! breadd.toml — the bread daemon config. Schema mirrors
//! breadd/src/core/config.rs (daemon, lua, modules, adapters, events,
//! notifications). Edited non-destructively via `commands::config`.

use serde::{Deserialize, Serialize};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("bread/breadd.toml")
}

#[derive(Serialize, Deserialize)]
pub struct BreadConfig {
    log_level: String,
    socket_path: String,
    lua_entry_point: String,
    lua_module_path: String,
    modules_builtin: bool,
    modules_disable: Vec<String>,
    adapter_hyprland: bool,
    adapter_udev: bool,
    udev_subsystems: Vec<String>,
    adapter_power: bool,
    power_poll_interval_secs: i64,
    adapter_network: bool,
    adapter_bluetooth: bool,
    dedup_window_ms: i64,
    notif_default_timeout_ms: i64,
    notif_default_urgency: String,
    notif_notify_send_path: String,
}

#[tauri::command]
pub fn get_bread_config() -> BreadConfig {
    let doc = config::load_doc(&config_path());
    BreadConfig {
        log_level: config::get_str(&doc, &["daemon", "log_level"]).unwrap_or_else(|| "info".into()),
        socket_path: config::get_str(&doc, &["daemon", "socket_path"]).unwrap_or_default(),
        lua_entry_point: config::get_str(&doc, &["lua", "entry_point"]).unwrap_or_default(),
        lua_module_path: config::get_str(&doc, &["lua", "module_path"]).unwrap_or_default(),
        modules_builtin: config::get_bool(&doc, &["modules", "builtin"]).unwrap_or(true),
        modules_disable: config::get_str_list(&doc, &["modules", "disable"]),
        adapter_hyprland: config::get_bool(&doc, &["adapters", "hyprland", "enabled"]).unwrap_or(true),
        adapter_udev: config::get_bool(&doc, &["adapters", "udev", "enabled"]).unwrap_or(true),
        udev_subsystems: config::get_str_list(&doc, &["adapters", "udev", "subsystems"]),
        adapter_power: config::get_bool(&doc, &["adapters", "power", "enabled"]).unwrap_or(true),
        power_poll_interval_secs: config::get_i64(&doc, &["adapters", "power", "poll_interval_secs"]).unwrap_or(30),
        adapter_network: config::get_bool(&doc, &["adapters", "network", "enabled"]).unwrap_or(true),
        adapter_bluetooth: config::get_bool(&doc, &["adapters", "bluetooth", "enabled"]).unwrap_or(true),
        dedup_window_ms: config::get_i64(&doc, &["events", "dedup_window_ms"]).unwrap_or(250),
        notif_default_timeout_ms: config::get_i64(&doc, &["notifications", "default_timeout_ms"]).unwrap_or(5000),
        notif_default_urgency: config::get_str(&doc, &["notifications", "default_urgency"]).unwrap_or_else(|| "normal".into()),
        notif_notify_send_path: config::get_str(&doc, &["notifications", "notify_send_path"]).unwrap_or_default(),
    }
}

/// Real, pickable module names for the "Disabled modules" field: breadd's
/// four compiled-in modules (bread/breadd/src/lua/mod.rs's `BUILTIN_*`
/// registry — these aren't files on disk, so a directory scan alone would
/// miss them) plus every `.lua` file actually sitting in the configured
/// module directory (the user's own custom widgets/modules — the common
/// case in practice, going by real configs seen in the wild).
#[tauri::command]
pub fn list_bread_modules() -> Vec<String> {
    let mut modules = vec![
        "bread.monitors".to_string(),
        "bread.devices".to_string(),
        "bread.workspaces".to_string(),
        "bread.binds".to_string(),
    ];

    let doc = config::load_doc(&config_path());
    let configured = config::get_str(&doc, &["lua", "module_path"]);
    let module_dir = expand_home(configured.as_deref().filter(|s| !s.is_empty()).unwrap_or("~/.config/bread/modules"));

    if let Ok(entries) = std::fs::read_dir(&module_dir) {
        let mut found: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("lua"))
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        found.sort();
        modules.extend(found);
    }

    modules
}

/// Expands a leading `~/` against `$HOME` — breadd's own config resolution
/// (`Config::lua_module_path`) does the same for this exact field.
fn expand_home(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

#[tauri::command]
pub fn save_bread_config(cfg: BreadConfig) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    config::set_str(&mut doc, &["daemon", "log_level"], &cfg.log_level);
    config::set_str_or_remove(&mut doc, &["daemon", "socket_path"], &cfg.socket_path);
    config::set_str_or_remove(&mut doc, &["lua", "entry_point"], &cfg.lua_entry_point);
    config::set_str_or_remove(&mut doc, &["lua", "module_path"], &cfg.lua_module_path);
    config::set_bool(&mut doc, &["modules", "builtin"], cfg.modules_builtin);
    config::set_str_list(&mut doc, &["modules", "disable"], &cfg.modules_disable);
    config::set_bool(&mut doc, &["adapters", "hyprland", "enabled"], cfg.adapter_hyprland);
    config::set_bool(&mut doc, &["adapters", "udev", "enabled"], cfg.adapter_udev);
    config::set_str_list(&mut doc, &["adapters", "udev", "subsystems"], &cfg.udev_subsystems);
    config::set_bool(&mut doc, &["adapters", "power", "enabled"], cfg.adapter_power);
    config::set_i64(&mut doc, &["adapters", "power", "poll_interval_secs"], cfg.power_poll_interval_secs);
    config::set_bool(&mut doc, &["adapters", "network", "enabled"], cfg.adapter_network);
    config::set_bool(&mut doc, &["adapters", "bluetooth", "enabled"], cfg.adapter_bluetooth);
    config::set_i64(&mut doc, &["events", "dedup_window_ms"], cfg.dedup_window_ms);
    config::set_i64(&mut doc, &["notifications", "default_timeout_ms"], cfg.notif_default_timeout_ms);
    config::set_str(&mut doc, &["notifications", "default_urgency"], &cfg.notif_default_urgency);
    config::set_str_or_remove(&mut doc, &["notifications", "notify_send_path"], &cfg.notif_notify_send_path);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}
