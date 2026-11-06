//! breadpad.toml: the breadpad notes/reminders config. Schema mirrors
//! breadpad-shared/src/config.rs (settings, model + model.ollama, reminders,
//! calendar). Edited non-destructively (calendar password + model paths
//! are preserved across saves).
//!
//! Reminder defaults must match breadpad itself, not a settings-app guess:
//! breadpad's serde defaults are `[reminders] default_morning = "08:00"` and
//! `[settings] snooze_options = ["15m", "1h", "tomorrow_morning"]`, and its
//! rule parser also uses 08:00 when the configured time cannot be parsed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config;

/// breadpad's own fallbacks (breadpad-shared/src/config.rs). Keep these in
/// sync with breadpad: showing a different default here and then saving would
/// overwrite the real values.
const BREADPAD_DEFAULT_MORNING: &str = "08:00";
const BREADPAD_DEFAULT_SNOOZE_OPTIONS: [&str; 3] = ["15m", "1h", "tomorrow_morning"];

fn config_path() -> PathBuf {
    config::config_dir().join("breadpad/breadpad.toml")
}

fn default_snooze_options() -> Vec<String> {
    BREADPAD_DEFAULT_SNOOZE_OPTIONS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Mirrors breadpad's `parse_rule_based`: split on ':' and take the first two
/// components, which must form a valid 24-hour time. Anything else makes
/// breadpad fall back to 08:00.
fn valid_morning_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(hour), Some(minute)) = (parts.next(), parts.next()) else {
        return false;
    };
    let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) else {
        return false;
    };
    hour < 24 && minute < 60
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

fn read_config(doc: &toml_edit::DocumentMut) -> BreadpadConfig {
    BreadpadConfig {
        default_type: config::get_str(doc, &["settings", "default_type"])
            .unwrap_or_else(|| "note".into()),
        workspace_tag: config::get_bool(doc, &["settings", "workspace_tag"]).unwrap_or(true),
        // Absent or malformed: breadpad's three defaults. An explicitly empty
        // array is a real choice and stays empty.
        snooze_options: config::get_str_list_strict(doc, &["settings", "snooze_options"])
            .unwrap_or_else(default_snooze_options),
        archive_after_days: config::get_i64(doc, &["settings", "archive_after_days"]).unwrap_or(30),
        model_path: config::get_str(doc, &["model", "path"]).unwrap_or_default(),
        tokenizer_path: config::get_str(doc, &["model", "tokenizer"]).unwrap_or_default(),
        ollama_enabled: config::get_bool(doc, &["model", "ollama", "enabled"]).unwrap_or(true),
        ollama_endpoint: config::get_str(doc, &["model", "ollama", "endpoint"]).unwrap_or_default(),
        ollama_model: config::get_str(doc, &["model", "ollama", "model"]).unwrap_or_default(),
        ollama_confidence_threshold: config::get_f64(
            doc,
            &["model", "ollama", "confidence_threshold"],
        )
        .unwrap_or(0.6),
        // Absent or unparseable: breadpad uses 08:00 for "tomorrow morning".
        reminders_default_morning: config::get_str(doc, &["reminders", "default_morning"])
            .filter(|value| valid_morning_time(value))
            .unwrap_or_else(|| BREADPAD_DEFAULT_MORNING.into()),
        reminders_missed_grace_minutes: config::get_i64(
            doc,
            &["reminders", "missed_grace_minutes"],
        )
        .unwrap_or(60),
        calendar_enabled: config::get_bool(doc, &["calendar", "enabled"]).unwrap_or(false),
        calendar_url: config::get_str(doc, &["calendar", "url"]).unwrap_or_default(),
        calendar_username: config::get_str(doc, &["calendar", "username"]).unwrap_or_default(),
        // Write-only to the webview, same as restic; never round-trip the secret.
        calendar_password: String::new(),
    }
}

#[tauri::command]
pub fn get_breadpad_config() -> BreadpadConfig {
    read_config(&config::load_doc(&config_path()))
}

fn apply_config(doc: &mut toml_edit::DocumentMut, cfg: &BreadpadConfig) {
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
    apply_calendar_password(doc, &cfg.calendar_password);
}

fn save_config_to(path: &Path, cfg: &BreadpadConfig) -> Result<(), String> {
    let mut doc = config::load_doc(path);
    apply_config(&mut doc, cfg);
    config::save_doc(path, &doc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_breadpad_config(cfg: BreadpadConfig) -> Result<(), String> {
    save_config_to(&config_path(), &cfg)
}

/// Empty incoming password keeps the existing secret (PasswordField is write-only).
fn apply_calendar_password(doc: &mut toml_edit::DocumentMut, incoming: &str) {
    if incoming.is_empty() {
        return;
    }
    config::set_str(doc, &["calendar", "password"], incoming);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-breadpad-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn empty_password_keeps_existing_secret() {
        let mut doc: toml_edit::DocumentMut =
            "[calendar]\npassword = \"secret\"\n".parse().unwrap();
        apply_calendar_password(&mut doc, "");
        assert_eq!(
            config::get_str(&doc, &["calendar", "password"]).as_deref(),
            Some("secret")
        );
        apply_calendar_password(&mut doc, "newpass");
        assert_eq!(
            config::get_str(&doc, &["calendar", "password"]).as_deref(),
            Some("newpass")
        );
    }

    #[test]
    fn reads_real_morning_and_snooze_values() {
        let doc: toml_edit::DocumentMut = r#"
[settings]
snooze_options = ["15m", "1h", "tomorrow_morning"]

[reminders]
default_morning = "08:00"
"#
        .parse()
        .unwrap();

        let cfg = read_config(&doc);
        assert_eq!(cfg.reminders_default_morning, "08:00");
        assert_eq!(
            cfg.snooze_options,
            vec![
                "15m".to_string(),
                "1h".to_string(),
                "tomorrow_morning".to_string()
            ]
        );
    }

    #[test]
    fn missing_file_uses_breadpad_defaults_without_creating_it() {
        let dir = tmp_dir("missing");
        let path = dir.join("breadpad.toml");
        assert!(!path.exists());

        let cfg = read_config(&config::load_doc(&path));
        assert_eq!(cfg.reminders_default_morning, "08:00");
        assert_eq!(cfg.snooze_options, default_snooze_options());
        assert!(!path.exists(), "reading the config must not write it");
    }

    #[test]
    fn invalid_values_fall_back_to_breadpad_defaults() {
        let doc: toml_edit::DocumentMut = r#"
[settings]
snooze_options = "15m"

[reminders]
default_morning = "quarter past eight"
"#
        .parse()
        .unwrap();
        let cfg = read_config(&doc);
        assert_eq!(cfg.reminders_default_morning, "08:00");
        assert_eq!(cfg.snooze_options, default_snooze_options());

        let mixed_snoozes: toml_edit::DocumentMut = "[settings]\nsnooze_options = [\"15m\", 1]\n"
            .parse()
            .unwrap();
        assert_eq!(
            read_config(&mixed_snoozes).snooze_options,
            default_snooze_options()
        );

        let bad_hour: toml_edit::DocumentMut = "[reminders]\ndefault_morning = \"24:00\"\n"
            .parse()
            .unwrap();
        assert_eq!(
            read_config(&bad_hour).reminders_default_morning,
            BREADPAD_DEFAULT_MORNING
        );
    }

    #[test]
    fn explicit_empty_snooze_list_stays_empty() {
        let doc: toml_edit::DocumentMut = "[settings]\nsnooze_options = []\n".parse().unwrap();
        assert!(read_config(&doc).snooze_options.is_empty());
    }

    #[test]
    fn save_roundtrip_preserves_unrelated_breadpad_config() {
        let dir = tmp_dir("roundtrip");
        let path = dir.join("breadpad.toml");
        let original = r#"# hand-written breadpad config
[settings]
default_type = "reminder"
workspace_tag = false
snooze_options = ["15m", "1h", "tomorrow_morning"]
archive_after_days = 14

[model]
path = "/opt/classifier.onnx"
ort_dylib_path = "/usr/lib/libonnxruntime.so"

[reminders]
default_morning = "08:00"
missed_grace_minutes = 45

[calendar]
enabled = true
url = "https://cloud.example/remote.php/dav/calendars/me/reminders/"
username = "me"
password = "do-not-touch"

[custom]
keep = "this too"
"#;
        std::fs::write(&path, original).unwrap();

        let mut cfg = read_config(&config::load_doc(&path));
        assert_eq!(cfg.reminders_default_morning, "08:00");
        cfg.reminders_default_morning = "09:15".into();
        cfg.snooze_options = vec!["30m".into(), "2h".into()];
        save_config_to(&path, &cfg).unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        for kept in [
            "# hand-written breadpad config",
            "default_type = \"reminder\"",
            "workspace_tag = false",
            "archive_after_days = 14",
            "path = \"/opt/classifier.onnx\"",
            "ort_dylib_path = \"/usr/lib/libonnxruntime.so\"",
            "missed_grace_minutes = 45",
            "enabled = true",
            "url = \"https://cloud.example/remote.php/dav/calendars/me/reminders/\"",
            "username = \"me\"",
            "password = \"do-not-touch\"",
            "keep = \"this too\"",
        ] {
            assert!(saved.contains(kept), "lost {kept:?} in:\n{saved}");
        }

        let doc: toml_edit::DocumentMut = saved.parse().unwrap();
        assert_eq!(
            config::get_str(&doc, &["reminders", "default_morning"]).as_deref(),
            Some("09:15")
        );
        assert_eq!(
            config::get_str_list(&doc, &["settings", "snooze_options"]),
            vec!["30m".to_string(), "2h".to_string()]
        );
        assert_eq!(
            config::get_str(&doc, &["calendar", "password"]).as_deref(),
            Some("do-not-touch")
        );

        // Reading the saved file back gives the same values the UI wrote.
        let back = read_config(&doc);
        assert_eq!(back.reminders_default_morning, "09:15");
        assert_eq!(
            back.snooze_options,
            vec!["30m".to_string(), "2h".to_string()]
        );
    }

    #[test]
    fn regression_old_seven_am_and_empty_snooze_display_cannot_return() {
        // The old reader defaulted to "7:00" and an empty list when the keys
        // were absent. Both must now match breadpad's real defaults.
        let cfg = read_config(&toml_edit::DocumentMut::new());
        assert_ne!(cfg.reminders_default_morning, "7:00");
        assert_eq!(cfg.reminders_default_morning, "08:00");
        assert!(!cfg.snooze_options.is_empty());
        assert_eq!(
            cfg.snooze_options,
            vec![
                "15m".to_string(),
                "1h".to_string(),
                "tomorrow_morning".to_string()
            ]
        );
    }
}
