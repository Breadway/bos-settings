//! breadsearch/config.toml: what the launcher's file indexer reads.

use std::path::PathBuf;

use crate::config;

pub struct Config {
    pub power_enabled: bool,
    pub run_on_battery: bool,
    pub backend: String,
    pub index_roots: Vec<String>,
    pub index_excludes: Vec<String>,
    pub index_extensions: Vec<String>,
    pub max_file_mb: f64,
    pub search_limit: i64,
    pub snippet_len: i64,
}

pub const BACKENDS: [(&str, &str); 5] = [
    ("cpu", "CPU"),
    ("npu", "NPU (AMD Ryzen AI)"),
    ("rocm", "AMD GPU (ROCm)"),
    ("cuda", "NVIDIA GPU (CUDA)"),
    ("openvino", "Intel GPU (OpenVINO)"),
];

pub fn path() -> PathBuf {
    config::config_dir().join("breadsearch/config.toml")
}

pub fn load() -> Config {
    let doc = config::load_doc(&path());
    Config {
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

pub fn save(cfg: &Config) -> Result<(), String> {
    let path = path();
    let mut doc = config::load_doc(&path);
    config::set_bool(&mut doc, &["power", "enabled"], cfg.power_enabled);
    config::set_bool(&mut doc, &["power", "run_on_battery"], cfg.run_on_battery);
    config::set_str(&mut doc, &["model", "backend"], cfg.backend.trim());
    config::set_str_list(&mut doc, &["index", "roots"], &cfg.index_roots);
    config::set_str_list(&mut doc, &["index", "excludes"], &cfg.index_excludes);
    config::set_str_list(&mut doc, &["index", "extensions"], &cfg.index_extensions);
    config::set_f64(&mut doc, &["index", "max_file_mb"], cfg.max_file_mb);
    config::set_i64(&mut doc, &["search", "limit"], cfg.search_limit);
    config::set_i64(&mut doc, &["search", "snippet_len"], cfg.snippet_len);
    config::save_doc(&path, &doc)
}

pub fn split_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_shipping_reader() {
        let doc = config::load_doc(&PathBuf::from("/nonexistent/breadsearch.toml"));
        assert_eq!(
            config::get_bool(&doc, &["power", "enabled"]).unwrap_or(true),
            true
        );
        assert_eq!(split_list("~/Notes, ~/Projects "), vec!["~/Notes", "~/Projects"]);
    }

    #[test]
    fn backend_labels_cover_codes() {
        assert!(BACKENDS.iter().any(|(code, _)| *code == "rocm"));
    }
}
