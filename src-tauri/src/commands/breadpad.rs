//! breadpad.toml — the breadpad notes/reminders config. Schema mirrors
//! breadpad-shared/src/config.rs (settings, model + model.ollama, reminders,
//! calendar). Edited non-destructively (calendar password + model paths
//! are preserved across saves).

use serde::{Deserialize, Serialize};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadpad/breadpad.toml")
}

#[derive(Serialize, Deserialize)]
pub struct BreadpadConfig {
    default_type: String,
    workspace_tag: bool,
    snooze_options: Vec<String>,
    archive_after_days: i64,
    model_path: String,
    tokenizer_path: String,
    ollama_enabled: bool,
    ollama_endpoint: String,
    ollama_model: String,
    ollama_confidence_threshold: f64,
    reminders_default_morning: String,
    reminders_missed_grace_minutes: i64,
    calendar_enabled: bool,
    calendar_url: String,
    calendar_username: String,
    calendar_password: String,
}

#[tauri::command]
pub fn get_breadpad_config() -> BreadpadConfig {
    let doc = config::load_doc(&config_path());
    BreadpadConfig {
        default_type: config::get_str(&doc, &["settings", "default_type"]).unwrap_or_else(|| "note".into()),
        workspace_tag: config::get_bool(&doc, &["settings", "workspace_tag"]).unwrap_or(true),
        snooze_options: config::get_str_list(&doc, &["settings", "snooze_options"]),
        archive_after_days: config::get_i64(&doc, &["settings", "archive_after_days"]).unwrap_or(30),
        model_path: config::get_str(&doc, &["model", "path"]).unwrap_or_default(),
        tokenizer_path: config::get_str(&doc, &["model", "tokenizer"]).unwrap_or_default(),
        ollama_enabled: config::get_bool(&doc, &["model", "ollama", "enabled"]).unwrap_or(true),
        ollama_endpoint: config::get_str(&doc, &["model", "ollama", "endpoint"]).unwrap_or_default(),
        ollama_model: config::get_str(&doc, &["model", "ollama", "model"]).unwrap_or_default(),
        ollama_confidence_threshold: config::get_f64(&doc, &["model", "ollama", "confidence_threshold"]).unwrap_or(0.6),
        reminders_default_morning: config::get_str(&doc, &["reminders", "default_morning"]).unwrap_or_else(|| "7:00".into()),
        reminders_missed_grace_minutes: config::get_i64(&doc, &["reminders", "missed_grace_minutes"]).unwrap_or(60),
        calendar_enabled: config::get_bool(&doc, &["calendar", "enabled"]).unwrap_or(false),
        calendar_url: config::get_str(&doc, &["calendar", "url"]).unwrap_or_default(),
        calendar_username: config::get_str(&doc, &["calendar", "username"]).unwrap_or_default(),
        calendar_password: config::get_str(&doc, &["calendar", "password"]).unwrap_or_default(),
    }
}

#[tauri::command]
pub fn save_breadpad_config(cfg: BreadpadConfig) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    config::set_str(&mut doc, &["settings", "default_type"], &cfg.default_type);
    config::set_bool(&mut doc, &["settings", "workspace_tag"], cfg.workspace_tag);
    config::set_str_list(&mut doc, &["settings", "snooze_options"], &cfg.snooze_options);
    config::set_i64(&mut doc, &["settings", "archive_after_days"], cfg.archive_after_days);
    config::set_str_or_remove(&mut doc, &["model", "path"], &cfg.model_path);
    config::set_str_or_remove(&mut doc, &["model", "tokenizer"], &cfg.tokenizer_path);
    config::set_bool(&mut doc, &["model", "ollama", "enabled"], cfg.ollama_enabled);
    config::set_str_or_remove(&mut doc, &["model", "ollama", "endpoint"], &cfg.ollama_endpoint);
    config::set_str_or_remove(&mut doc, &["model", "ollama", "model"], &cfg.ollama_model);
    config::set_f64(&mut doc, &["model", "ollama", "confidence_threshold"], cfg.ollama_confidence_threshold);
    config::set_str_or_remove(&mut doc, &["reminders", "default_morning"], &cfg.reminders_default_morning);
    config::set_i64(&mut doc, &["reminders", "missed_grace_minutes"], cfg.reminders_missed_grace_minutes);
    config::set_bool(&mut doc, &["calendar", "enabled"], cfg.calendar_enabled);
    config::set_str_or_remove(&mut doc, &["calendar", "url"], &cfg.calendar_url);
    config::set_str_or_remove(&mut doc, &["calendar", "username"], &cfg.calendar_username);
    config::set_str_or_remove(&mut doc, &["calendar", "password"], &cfg.calendar_password);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}
