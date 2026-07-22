//! breadsearch/config.toml — semantic search indexer (breadmill) + GUI.
//! Schema mirrors breadsearch-shared::Config ([index], [search], [model], [power]).

use serde::{Deserialize, Serialize};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadsearch/config.toml")
}

#[derive(Serialize, Deserialize)]
pub struct BreadsearchConfig {
    power_enabled: bool,
    run_on_battery: bool,
    backend: String,
    index_roots: Vec<String>,
    index_excludes: Vec<String>,
    index_extensions: Vec<String>,
    max_file_mb: f64,
    search_limit: i64,
    snippet_len: i64,
}

#[tauri::command]
pub fn get_breadsearch_config() -> BreadsearchConfig {
    let doc = config::load_doc(&config_path());
    BreadsearchConfig {
        power_enabled: config::get_bool(&doc, &["power", "enabled"]).unwrap_or(true),
        run_on_battery: config::get_bool(&doc, &["power", "run_on_battery"]).unwrap_or(false),
        backend: config::get_str(&doc, &["model", "backend"]).unwrap_or_else(|| "cpu".into()),
        index_roots: config::get_str_list(&doc, &["index", "roots"]),
        index_excludes: config::get_str_list(&doc, &["index", "excludes"]),
        index_extensions: config::get_str_list(&doc, &["index", "extensions"]),
        max_file_mb: config::get_f64(&doc, &["index", "max_file_mb"]).unwrap_or(10.0),
        search_limit: config::get_i64(&doc, &["search", "limit"]).unwrap_or(10),
        snippet_len: config::get_i64(&doc, &["search", "snippet_len"]).unwrap_or(200),
    }
}

#[tauri::command]
pub fn save_breadsearch_config(cfg: BreadsearchConfig) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    config::set_bool(&mut doc, &["power", "enabled"], cfg.power_enabled);
    config::set_bool(&mut doc, &["power", "run_on_battery"], cfg.run_on_battery);
    config::set_str(&mut doc, &["model", "backend"], &cfg.backend);
    config::set_str_list(&mut doc, &["index", "roots"], &cfg.index_roots);
    config::set_str_list(&mut doc, &["index", "excludes"], &cfg.index_excludes);
    config::set_str_list(&mut doc, &["index", "extensions"], &cfg.index_extensions);
    config::set_f64(&mut doc, &["index", "max_file_mb"], cfg.max_file_mb);
    config::set_i64(&mut doc, &["search", "limit"], cfg.search_limit);
    config::set_i64(&mut doc, &["search", "snippet_len"], cfg.snippet_len);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}
