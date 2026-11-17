//! breadram/breadram.toml, the memory daemon's own configuration.
//!
//! The file may hold keys and comments this page does not model, so writes
//! load the document, replace only the modelled leaves, and write it back
//! atomically with a `.bak` of the previous contents. A file that exists but
//! does not parse is reported instead of being overwritten.

use std::path::{Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item, Table};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Provider {
    pub name: String,
    pub kind: String,
    pub url: String,
    pub scope: String,
    pub allow_automation: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub mode: Option<String>,
    pub allowed_actions: Vec<String>,
    pub protected: Vec<String>,
    pub providers: Vec<Provider>,
    pub jobs_enabled: Option<bool>,
    pub jobs_total_budget_gib: Option<f64>,
    pub jobs_default_estimate_gib: Option<f64>,
    pub jobs_auto_launch: Option<bool>,
}

pub fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("breadram/breadram.toml")
}

pub fn load() -> Result<Config, String> {
    load_from(&path())
}

fn read_doc(path: &Path) -> Result<DocumentMut, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    text.parse::<DocumentMut>()
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn read_item<'a>(doc: &'a DocumentMut, path: &[&str]) -> Option<&'a Item> {
    let (last, parents) = path.split_last()?;
    let mut table = doc.as_table();
    for key in parents {
        table = table.get(key)?.as_table()?;
    }
    table.get(last)
}

fn read_str(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    read_item(doc, path)?.as_str().map(str::to_string)
}

fn read_bool(doc: &DocumentMut, path: &[&str]) -> Option<bool> {
    read_item(doc, path)?.as_bool()
}

fn read_i64(doc: &DocumentMut, path: &[&str]) -> Option<i64> {
    read_item(doc, path)?.as_integer()
}

fn read_strings(item: Option<&Item>) -> Vec<String> {
    item.and_then(Item::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Lists may sit in a table, as the daemon writes them, or flat in a hand
/// written file. Both spellings read; writes use the table.
fn read_list(doc: &DocumentMut, table: &str, key: &str) -> Vec<String> {
    let nested = read_strings(
        doc.get(table)
            .and_then(Item::as_table)
            .and_then(|t| t.get(key)),
    );
    if nested.is_empty() {
        read_strings(doc.get(key))
    } else {
        nested
    }
}

fn read_providers(doc: &DocumentMut) -> Vec<Provider> {
    doc.get("providers")
        .and_then(Item::as_array_of_tables)
        .map(|tables| {
            tables
                .iter()
                .map(|table| Provider {
                    name: table
                        .get("name")
                        .and_then(Item::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    kind: table
                        .get("kind")
                        .and_then(Item::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    url: table
                        .get("url")
                        .and_then(Item::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    scope: table
                        .get("scope")
                        .and_then(Item::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    allow_automation: table
                        .get("allow_automation")
                        .and_then(Item::as_bool)
                        .unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn gib(bytes: Option<i64>) -> Option<f64> {
    bytes.map(|value| value as f64 / 1024.0 / 1024.0 / 1024.0)
}

pub fn load_from(path: &Path) -> Result<Config, String> {
    let doc = read_doc(path)?;
    Ok(Config {
        mode: read_str(&doc, &["mode"]),
        allowed_actions: read_list(&doc, "automation", "allowed_actions"),
        protected: read_list(&doc, "protection", "protected"),
        providers: read_providers(&doc),
        jobs_enabled: read_bool(&doc, &["jobs", "enabled"]),
        jobs_total_budget_gib: gib(read_i64(&doc, &["jobs", "total_budget_bytes"])),
        jobs_default_estimate_gib: gib(read_i64(&doc, &["jobs", "default_estimate_bytes"])),
        jobs_auto_launch: read_bool(&doc, &["jobs", "auto_launch"]),
    })
}

fn gib_to_bytes(gib: f64, what: &str) -> Result<i64, String> {
    if !gib.is_finite() {
        return Err(format!("{what} must be a real number"));
    }
    if gib < 0.0 {
        return Err(format!("{what} cannot be negative"));
    }
    if gib > 1_048_576.0 {
        return Err(format!("{what} above one pebibyte is not a real budget"));
    }
    Ok((gib * 1024.0 * 1024.0 * 1024.0).round() as i64)
}

fn string_array(items: &[String]) -> Item {
    let mut array = Array::new();
    for item in items {
        array.push(item.as_str());
    }
    Item::Value(toml_edit::Value::Array(array))
}

fn table_with(table: Option<&Table>, key: &str, item: Item) -> Item {
    let mut table = table.cloned().unwrap_or_default();
    table.insert(key, item);
    Item::Table(table)
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!("{} has no parent directory", path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "breadram.toml".into()),
        std::process::id()
    ));
    std::fs::write(&temp, text).map_err(|error| format!("{}: {error}", temp.display()))?;
    if path.exists() {
        let backup = path.with_extension("toml.bak");
        std::fs::copy(path, &backup)
            .map_err(|error| format!("{}: {error}", backup.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("{}: {error}", path.display())
    })
}

pub fn save_to(path: &Path, cfg: &Config) -> Result<(), String> {
    if let Some(mode) = cfg.mode.as_deref() {
        if !matches!(mode, "monitor" | "automatic") {
            return Err(format!("unknown management mode: {mode}"));
        }
    }
    // Validate budgets before loading or touching anything, so a refused
    // save leaves the file exactly as it was.
    let total = cfg
        .jobs_total_budget_gib
        .map(|gib| gib_to_bytes(gib, "job budget"))
        .transpose()?;
    let estimate = cfg
        .jobs_default_estimate_gib
        .map(|gib| gib_to_bytes(gib, "job estimate"))
        .transpose()?;

    let mut doc = read_doc(path)?;
    if let Some(mode) = cfg.mode.as_deref() {
        doc.insert("mode", value(mode));
    }
    doc.as_table_mut().remove("allowed_actions");
    doc.as_table_mut().remove("protected");
    let automation = table_with(
        doc.get("automation").and_then(Item::as_table),
        "allowed_actions",
        string_array(&cfg.allowed_actions),
    );
    doc.insert("automation", automation);
    let protection = table_with(
        doc.get("protection").and_then(Item::as_table),
        "protected",
        string_array(&cfg.protected),
    );
    doc.insert("protection", protection);
    if let Some(enabled) = cfg.jobs_enabled {
        set_value(&mut doc, &["jobs", "enabled"], value(enabled));
    }
    if let Some(bytes) = total {
        set_value(&mut doc, &["jobs", "total_budget_bytes"], value(bytes));
    }
    if let Some(bytes) = estimate {
        set_value(&mut doc, &["jobs", "default_estimate_bytes"], value(bytes));
    }
    if let Some(auto) = cfg.jobs_auto_launch {
        set_value(&mut doc, &["jobs", "auto_launch"], value(auto));
    }
    // Providers merge by name so hook commands, credential references and
    // other unmodelled keys survive. Removed providers are removed on purpose.
    let mut tables = toml_edit::ArrayOfTables::new();
    for provider in &cfg.providers {
        let mut table = doc
            .get("providers")
            .and_then(Item::as_array_of_tables)
            .and_then(|existing| {
                existing
                    .iter()
                    .find(|table| {
                        table.get("name").and_then(Item::as_str)
                            == Some(provider.name.as_str())
                    })
                    .cloned()
            })
            .unwrap_or_default();
        table.insert("name", value(provider.name.clone()));
        table.insert("kind", value(provider.kind.clone()));
        table.insert("url", value(provider.url.clone()));
        table.insert("scope", value(provider.scope.clone()));
        table.insert("allow_automation", value(provider.allow_automation));
        tables.push(table);
    }
    doc.insert("providers", Item::ArrayOfTables(tables));
    write_atomic(path, &doc.to_string())
}

fn set_value(doc: &mut DocumentMut, path: &[&str], item: Item) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut table = doc.as_table_mut();
    for key in parents {
        let entry = table
            .entry(key)
            .or_insert_with(|| Item::Table(Table::new()));
        if !entry.is_table() {
            *entry = Item::Table(Table::new());
        }
        table = entry.as_table_mut().expect("just ensured a table");
    }
    table.insert(last, item);
}

pub fn apply_profile_to(path: &Path, profile: &str) -> Result<(), String> {
    let (mode, actions, protected): (&str, &[&str], &[&str]) = match profile {
        "monitor" => ("monitor", &[], &[]),
        "desktop" => (
            "automatic",
            &[
                "reclaim_protection",
                "cgroup_soft_limit",
                "unload_idle_model",
            ],
            &["exe:Hyprland", "unit:systemd-logind.service"],
        ),
        "headless-server" => (
            "automatic",
            &[
                "cgroup_soft_limit",
                "reclaim_protection",
                "queue_background_job",
            ],
            &[],
        ),
        other => return Err(format!("unknown profile: {other}")),
    };
    let mut cfg = load_from(path)?;
    cfg.mode = Some(mode.to_string());
    cfg.allowed_actions = actions.iter().map(|value| value.to_string()).collect();
    cfg.protected = protected.iter().map(|value| value.to_string()).collect();
    save_to(path, &cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-breadram-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("breadram.toml")
    }

    const SAMPLE: &str = "\
# operator comment
version = 1
mode = \"automatic\"
custom_key = \"keep me\"

[automation]
allowed_actions = [\"reclaim_protection\", \"sleep_model\"]

[protection]
protected = [\"exe:Hyprland\"]

[history]
max_records = 10000

[[providers]]
name = \"llama-server\"
kind = \"llama_cpp\"
url = \"http://127.0.0.1:7301\"
scope = \"unit:llamacpp.service\"
allow_automation = true
command = [\"/usr/bin/hook\"]
";

    #[test]
    fn reads_the_real_daemon_shape() {
        let path = scratch("read");
        std::fs::write(&path, SAMPLE).unwrap();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.mode.as_deref(), Some("automatic"));
        assert_eq!(
            cfg.allowed_actions,
            vec!["reclaim_protection".to_string(), "sleep_model".to_string()]
        );
        assert_eq!(cfg.protected, vec!["exe:Hyprland".to_string()]);
        assert_eq!(cfg.providers.len(), 1);
        assert_eq!(cfg.providers[0].kind, "llama_cpp");
        assert!(cfg.providers[0].allow_automation);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn flat_lists_read_too() {
        let path = scratch("flat");
        std::fs::write(
            &path,
            "mode = \"monitor\"\nallowed_actions = [\"sleep_model\"]\nprotected = []\n",
        )
        .unwrap();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.mode.as_deref(), Some("monitor"));
        assert_eq!(cfg.allowed_actions, vec!["sleep_model".to_string()]);
        assert!(cfg.protected.is_empty());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_preserves_unmodelled_keys_and_comments() {
        let path = scratch("save");
        std::fs::write(&path, SAMPLE).unwrap();
        let mut cfg = load_from(&path).unwrap();
        cfg.mode = Some("monitor".to_string());
        cfg.allowed_actions = vec!["cgroup_soft_limit".to_string()];
        cfg.protected = vec!["unit:systemd-logind.service".to_string()];
        save_to(&path, &cfg).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# operator comment"));
        assert!(text.contains("custom_key = \"keep me\""));
        assert!(text.contains("max_records = 10000"));
        assert!(text.contains("command = [\"/usr/bin/hook\"]"));
        assert!(text.contains("mode = \"monitor\""));
        assert!(text.contains("[automation]"));
        assert!(text.contains("[protection]"));
        assert!(!text.contains("allowed_actions = [\"reclaim_protection\"]"));
        let reloaded = load_from(&path).unwrap();
        assert_eq!(reloaded.allowed_actions, vec!["cgroup_soft_limit".to_string()]);
        assert_eq!(
            reloaded.protected,
            vec!["unit:systemd-logind.service".to_string()]
        );
        assert!(path.with_extension("toml.bak").exists(), "a backup is kept");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn profiles_resolve_to_the_documented_settings() {
        let path = scratch("profiles");
        std::fs::write(&path, "").unwrap();
        apply_profile_to(&path, "desktop").unwrap();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.mode.as_deref(), Some("automatic"));
        assert!(cfg
            .allowed_actions
            .contains(&"unload_idle_model".to_string()));
        assert!(cfg.protected.contains(&"exe:Hyprland".to_string()));
        apply_profile_to(&path, "monitor").unwrap();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.mode.as_deref(), Some("monitor"));
        assert!(cfg.allowed_actions.is_empty());
        assert!(apply_profile_to(&path, "nope").is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn refused_saves_change_nothing() {
        let path = scratch("refused");
        std::fs::write(&path, SAMPLE).unwrap();
        let mut cfg = load_from(&path).unwrap();
        cfg.mode = Some("ludicrous".to_string());
        assert!(save_to(&path, &cfg).is_err());
        cfg.mode = Some("automatic".to_string());
        cfg.jobs_total_budget_gib = Some(f64::NAN);
        assert!(save_to(&path, &cfg).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), SAMPLE);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_missing_file_reads_as_defaults() {
        let path = scratch("missing");
        std::fs::remove_file(&path).ok();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg, Config::default());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
