//! Lock screen (breadlock) and greeter (breadgreet).
//!
//! Super+L is `loginctl lock-session`; hypridle's lock_cmd / idle listener
//! then starts breadlock. This panel edits `~/.config/breadlock/breadlock.toml`
//! (appearance + fail timeout) — it does not, and cannot, configure PAM.
//! breadgreet is the greetd greeter; its live config is typically
//! `/etc/greetd/breadgreet.toml` (system, owned by the greeter user) and is
//! not written from here.

use serde::{Deserialize, Serialize};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadlock/breadlock.toml")
}

const EXAMPLE_CANDIDATES: &[&str] = &[
    "/usr/share/doc/breadlock/breadlock.example.toml",
    "/usr/share/breadlock/breadlock.example.toml",
    "/usr/share/doc/breadlock/examples/breadlock.example.toml",
];

#[derive(Serialize, Deserialize)]
pub struct BreadlockConfig {
    background_mode: String,
    background_path: String,
    background_blur: bool,
    clock_format: String,
    font_family: String,
    fail_timeout_ms: i64,
}

impl Default for BreadlockConfig {
    fn default() -> Self {
        Self {
            background_mode: "color".into(),
            background_path: String::new(),
            background_blur: false,
            clock_format: "%H:%M".into(),
            font_family: "Varela Round".into(),
            fail_timeout_ms: 800,
        }
    }
}

#[tauri::command]
pub fn get_breadlock_config() -> BreadlockConfig {
    let doc = config::load_doc(&config_path());
    let mut cfg = BreadlockConfig::default();
    if let Some(mode) = config::get_str(&doc, &["background", "mode"]) {
        cfg.background_mode = mode;
    }
    if let Some(path) = config::get_str(&doc, &["background", "path"]) {
        cfg.background_path = path;
    }
    if let Some(blur) = config::get_bool(&doc, &["background", "blur"]) {
        cfg.background_blur = blur;
    }
    if let Some(fmt) = config::get_str(&doc, &["clock", "format"]) {
        cfg.clock_format = fmt;
    }
    if let Some(family) = config::get_str(&doc, &["font", "family"]) {
        cfg.font_family = family;
    }
    if let Some(ms) = config::get_i64(&doc, &["input", "fail_timeout_ms"]) {
        cfg.fail_timeout_ms = ms;
    }
    cfg
}

#[tauri::command]
pub fn save_breadlock_config(cfg: BreadlockConfig) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    let mode = if cfg.background_mode == "image" {
        "image"
    } else {
        "color"
    };
    config::set_str(&mut doc, &["background", "mode"], mode);
    config::set_str_or_remove(&mut doc, &["background", "path"], &cfg.background_path);
    config::set_bool(&mut doc, &["background", "blur"], cfg.background_blur);
    config::set_str(&mut doc, &["clock", "format"], &cfg.clock_format);
    config::set_str(&mut doc, &["font", "family"], &cfg.font_family);
    config::set_i64(
        &mut doc,
        &["input", "fail_timeout_ms"],
        cfg.fail_timeout_ms.max(0),
    );
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}

/// First existing packaged example, if any — the panel links this rather
/// than pretending the in-app editor is the whole schema.
#[tauri::command]
pub fn breadlock_example_path() -> Option<String> {
    EXAMPLE_CANDIDATES
        .iter()
        .find(|p| std::path::Path::new(p).is_file())
        .map(|p| p.to_string())
}

fn open_in_editor(path: &std::path::Path) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let _ = std::process::Command::new("kitty")
        .args(["-e", &editor])
        .arg(path)
        .spawn();
}

#[tauri::command]
pub fn open_breadlock_config() {
    open_in_editor(&config_path());
}

#[tauri::command]
pub fn open_breadlock_example() {
    if let Some(path) = breadlock_example_path() {
        open_in_editor(std::path::Path::new(&path));
    }
}

/// Super+L / hypridle path: `loginctl lock-session` → compositor lock
/// protocol → breadlock. Fire-and-forget; locking the live session is the
/// point of the button.
#[tauri::command]
pub fn lock_session() {
    let _ = std::process::Command::new("loginctl")
        .arg("lock-session")
        .spawn();
}
