//! breadpad/breadpad.toml: notes, reminders and the local model.

use std::path::{Path, PathBuf};

use crate::config;

pub const DEFAULT_MORNING: &str = "08:00";
pub const DEFAULT_SNOOZE_OPTIONS: [&str; 3] = ["15m", "1h", "tomorrow_morning"];

pub struct Config {
    pub default_type: String,
    pub workspace_tag: bool,
    pub snooze_options: Vec<String>,
    pub archive_after_days: i64,
    pub model_path: String,
    pub tokenizer_path: String,
    pub ollama_enabled: bool,
    pub ollama_endpoint: String,
    pub ollama_model: String,
    pub ollama_confidence_threshold: f64,
    pub reminders_default_morning: String,
    pub reminders_missed_grace_minutes: i64,
    pub calendar_enabled: bool,
    pub calendar_url: String,
    pub calendar_username: String,
    /// Write-only, same as the shipping app: an empty value keeps the stored
    /// password instead of clearing it.
    pub calendar_password: String,
}

pub fn path() -> PathBuf {
    config::config_dir().join("breadpad/breadpad.toml")
}

fn default_snooze_options() -> Vec<String> {
    DEFAULT_SNOOZE_OPTIONS
        .iter()
        .map(|item| item.to_string())
        .collect()
}

pub fn valid_morning_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(hour), Some(minute)) = (parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) else {
        return false;
    };
    hour < 24 && minute < 60
}

pub fn read_config(doc: &toml_edit::DocumentMut) -> Config {
    Config {
        default_type: config::get_str(doc, &["settings", "default_type"])
            .unwrap_or_else(|| "note".into()),
        workspace_tag: config::get_bool(doc, &["settings", "workspace_tag"]).unwrap_or(true),
        snooze_options: config::get_str_list_strict(doc, &["settings", "snooze_options"])
            .unwrap_or_else(default_snooze_options),
        archive_after_days: config::get_i64(doc, &["settings", "archive_after_days"])
            .unwrap_or(30),
        model_path: config::get_str(doc, &["model", "path"]).unwrap_or_default(),
        tokenizer_path: config::get_str(doc, &["model", "tokenizer"]).unwrap_or_default(),
        ollama_enabled: config::get_bool(doc, &["model", "ollama", "enabled"]).unwrap_or(true),
        ollama_endpoint: config::get_str(doc, &["model", "ollama", "endpoint"])
            .unwrap_or_default(),
        ollama_model: config::get_str(doc, &["model", "ollama", "model"]).unwrap_or_default(),
        ollama_confidence_threshold: config::get_f64(
            doc,
            &["model", "ollama", "confidence_threshold"],
        )
        .unwrap_or(0.6),
        reminders_default_morning: config::get_str(doc, &["reminders", "default_morning"])
            .filter(|value| valid_morning_time(value))
            .unwrap_or_else(|| DEFAULT_MORNING.into()),
        reminders_missed_grace_minutes: config::get_i64(
            doc,
            &["reminders", "missed_grace_minutes"],
        )
        .unwrap_or(60),
        calendar_enabled: config::get_bool(doc, &["calendar", "enabled"]).unwrap_or(false),
        calendar_url: config::get_str(doc, &["calendar", "url"]).unwrap_or_default(),
        calendar_username: config::get_str(doc, &["calendar", "username"]).unwrap_or_default(),
        calendar_password: String::new(),
    }
}

pub fn load() -> Config {
    read_config(&config::load_doc(&path()))
}

fn apply(doc: &mut toml_edit::DocumentMut, cfg: &Config) {
    config::set_str(doc, &["settings", "default_type"], &cfg.default_type);
    config::set_bool(doc, &["settings", "workspace_tag"], cfg.workspace_tag);
    config::set_str_list(doc, &["settings", "snooze_options"], &cfg.snooze_options);
    config::set_i64(
        doc,
        &["settings", "archive_after_days"],
        cfg.archive_after_days,
    );
    config::set_str_or_remove(doc, &["model", "path"], &cfg.model_path);
    config::set_str_or_remove(doc, &["model", "tokenizer"], &cfg.tokenizer_path);
    config::set_bool(doc, &["model", "ollama", "enabled"], cfg.ollama_enabled);
    config::set_str_or_remove(doc, &["model", "ollama", "endpoint"], &cfg.ollama_endpoint);
    config::set_str_or_remove(doc, &["model", "ollama", "model"], &cfg.ollama_model);
    config::set_f64(
        doc,
        &["model", "ollama", "confidence_threshold"],
        cfg.ollama_confidence_threshold,
    );
    config::set_str_or_remove(
        doc,
        &["reminders", "default_morning"],
        &cfg.reminders_default_morning,
    );
    config::set_i64(
        doc,
        &["reminders", "missed_grace_minutes"],
        cfg.reminders_missed_grace_minutes,
    );
    config::set_bool(doc, &["calendar", "enabled"], cfg.calendar_enabled);
    config::set_str_or_remove(doc, &["calendar", "url"], &cfg.calendar_url);
    config::set_str_or_remove(doc, &["calendar", "username"], &cfg.calendar_username);
    if !cfg.calendar_password.is_empty() {
        config::set_str(doc, &["calendar", "password"], &cfg.calendar_password);
    }
}

pub fn save_to(path: &Path, cfg: &Config) -> Result<(), String> {
    let mut doc = config::load_doc(path);
    apply(&mut doc, cfg);
    config::save_doc(path, &doc)
}

pub fn save(cfg: &Config) -> Result<(), String> {
    save_to(&path(), cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_values_use_breadpad_defaults() {
        let cfg = read_config(&toml_edit::DocumentMut::new());
        assert_eq!(cfg.reminders_default_morning, "08:00");
        assert_eq!(cfg.snooze_options, default_snooze_options());
        assert_eq!(cfg.calendar_password, "");
    }

    #[test]
    fn empty_password_keeps_the_stored_secret() {
        let mut doc: toml_edit::DocumentMut =
            "[calendar]\npassword = \"secret\"\n".parse().unwrap();
        let mut cfg = read_config(&doc);
        cfg.calendar_password = String::new();
        apply(&mut doc, &cfg);
        assert_eq!(
            config::get_str(&doc, &["calendar", "password"]).as_deref(),
            Some("secret")
        );
    }

    #[test]
    fn bad_morning_falls_back() {
        let doc: toml_edit::DocumentMut = "[reminders]\ndefault_morning = \"24:00\"\n"
            .parse()
            .unwrap();
        assert_eq!(
            read_config(&doc).reminders_default_morning,
            DEFAULT_MORNING
        );
    }
}
