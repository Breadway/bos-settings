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

pub fn region_to_clipboard() -> Result<(), String> {
    Command::new("breadshot")
        .args(["region", "--clipboard-only"])
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
