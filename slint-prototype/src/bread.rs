//! breadd.toml, the bread daemon configuration.

use std::path::{Path, PathBuf};

use toml_edit::{value, DocumentMut, Item, Table};

#[derive(Clone, Debug)]
pub struct Config {
    pub log_level: String,
    pub socket_path: String,
    pub lua_entry_point: String,
    pub lua_module_path: String,
    pub modules_builtin: bool,
    pub modules_disable: String,
    pub adapter_hyprland: bool,
    pub adapter_udev: bool,
    pub adapter_power: bool,
    pub adapter_network: bool,
    pub adapter_bluetooth: bool,
    pub dedup_window_ms: i64,
    pub notif_default_timeout_ms: i64,
    pub notif_default_urgency: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".into(),
            socket_path: String::new(),
            lua_entry_point: String::new(),
            lua_module_path: String::new(),
            modules_builtin: true,
            modules_disable: String::new(),
            adapter_hyprland: true,
            adapter_udev: true,
            adapter_power: true,
            adapter_network: true,
            adapter_bluetooth: true,
            dedup_window_ms: 250,
            notif_default_timeout_ms: 5000,
            notif_default_urgency: "normal".into(),
        }
    }
}

pub fn path() -> PathBuf {
    config_dir().join("bread/breadd.toml")
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
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

fn string_at(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    let mut item = doc.get(path.first()?)?;
    for key in &path[1..] {
        item = item.as_table()?.get(key)?;
    }
    item.as_str().map(str::to_string)
}

fn bool_at(doc: &DocumentMut, path: &[&str]) -> Option<bool> {
    let mut item = doc.get(path.first()?)?;
    for key in &path[1..] {
        item = item.as_table()?.get(key)?;
    }
    item.as_bool()
}

fn int_at(doc: &DocumentMut, path: &[&str]) -> Option<i64> {
    let mut item = doc.get(path.first()?)?;
    for key in &path[1..] {
        item = item.as_table()?.get(key)?;
    }
    item.as_integer()
}

fn list_at(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    let mut item = doc.get(path.first()?)?;
    for key in &path[1..] {
        item = item.as_table()?.get(key)?;
    }
    Some(
        item.as_array()?
            .iter()
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

pub fn load() -> Config {
    let Ok(doc) = read_doc(&path()) else {
        return Config::default();
    };
    let mut cfg = Config::default();
    if let Some(value) = string_at(&doc, &["daemon", "log_level"]) {
        cfg.log_level = value;
    }
    if let Some(value) = string_at(&doc, &["daemon", "socket_path"]) {
        cfg.socket_path = value;
    }
    if let Some(value) = string_at(&doc, &["lua", "entry_point"]) {
        cfg.lua_entry_point = value;
    }
    if let Some(value) = string_at(&doc, &["lua", "module_path"]) {
        cfg.lua_module_path = value;
    }
    if let Some(value) = bool_at(&doc, &["modules", "builtin"]) {
        cfg.modules_builtin = value;
    }
    if let Some(value) = list_at(&doc, &["modules", "disable"]) {
        cfg.modules_disable = value;
    }
    for (key, target) in [
        ("hyprland", &mut cfg.adapter_hyprland),
        ("udev", &mut cfg.adapter_udev),
        ("power", &mut cfg.adapter_power),
        ("network", &mut cfg.adapter_network),
        ("bluetooth", &mut cfg.adapter_bluetooth),
    ] {
        if let Some(value) = bool_at(&doc, &["adapters", key, "enabled"]) {
            *target = value;
        }
    }
    if let Some(value) = int_at(&doc, &["events", "dedup_window_ms"]) {
        cfg.dedup_window_ms = value;
    }
    if let Some(value) = int_at(&doc, &["notifications", "default_timeout_ms"]) {
        cfg.notif_default_timeout_ms = value;
    }
    if let Some(value) = string_at(&doc, &["notifications", "default_urgency"]) {
        cfg.notif_default_urgency = value;
    }
    cfg
}

fn table_with(table: Option<&Table>, key: &str, item: Item) -> Table {
    let mut table = table.cloned().unwrap_or_default();
    table.insert(key, item);
    table
}

fn put_string(doc: &mut DocumentMut, section: &str, key: &str, text: &str) {
    let table = table_with(
        doc.get(section).and_then(Item::as_table),
        key,
        value(text.to_string()),
    );
    doc.insert(section, Item::Table(table));
}

fn put_bool(doc: &mut DocumentMut, section: &str, key: &str, on: bool) {
    let table = table_with(doc.get(section).and_then(Item::as_table), key, value(on));
    doc.insert(section, Item::Table(table));
}

fn put_nested_bool(doc: &mut DocumentMut, outer: &str, inner: &str, on: bool) {
    let nested = table_with(
        doc.get(outer)
            .and_then(Item::as_table)
            .and_then(|table| table.get(inner))
            .and_then(Item::as_table),
        "enabled",
        value(on),
    );
    let outer_table = table_with(
        doc.get(outer).and_then(Item::as_table),
        inner,
        Item::Table(nested),
    );
    doc.insert(outer, Item::Table(outer_table));
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let path = path();
    let mut doc = read_doc(&path)?;
    put_string(&mut doc, "daemon", "log_level", &cfg.log_level);
    put_string(&mut doc, "daemon", "socket_path", &cfg.socket_path);
    put_string(&mut doc, "lua", "entry_point", &cfg.lua_entry_point);
    put_string(&mut doc, "lua", "module_path", &cfg.lua_module_path);
    put_bool(&mut doc, "modules", "builtin", cfg.modules_builtin);
    let modules = cfg
        .modules_disable
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| toml_edit::Value::from(name.to_string()))
        .collect::<Vec<_>>();
    let modules_table = table_with(
        doc.get("modules").and_then(Item::as_table),
        "disable",
        Item::Value(toml_edit::Value::Array(toml_edit::Array::from_iter(
            modules,
        ))),
    );
    doc.insert("modules", Item::Table(modules_table));
    for (key, on) in [
        ("hyprland", cfg.adapter_hyprland),
        ("udev", cfg.adapter_udev),
        ("power", cfg.adapter_power),
        ("network", cfg.adapter_network),
        ("bluetooth", cfg.adapter_bluetooth),
    ] {
        put_nested_bool(&mut doc, "adapters", key, on);
    }
    let events = table_with(
        doc.get("events").and_then(Item::as_table),
        "dedup_window_ms",
        value(cfg.dedup_window_ms),
    );
    doc.insert("events", Item::Table(events));
    let mut notifications = table_with(
        doc.get("notifications").and_then(Item::as_table),
        "default_timeout_ms",
        value(cfg.notif_default_timeout_ms),
    );
    notifications.insert(
        "default_urgency",
        value(cfg.notif_default_urgency.clone()),
    );
    doc.insert("notifications", Item::Table(notifications));
    let Some(parent) = path.parent() else {
        return Err("config path has no parent".into());
    };
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(&path, doc.to_string()).map_err(|error| error.to_string())
}

pub fn modules() -> Vec<String> {
    let mut modules = vec![
        "bread.monitors".to_string(),
        "bread.devices".to_string(),
        "bread.workspaces".to_string(),
        "bread.binds".to_string(),
    ];
    let cfg = load();
    let dir = if cfg.lua_module_path.is_empty() {
        config_dir().join("bread/modules")
    } else if let Some(rest) = cfg.lua_module_path.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(rest)
    } else {
        PathBuf::from(&cfg.lua_module_path)
    };
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut found: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("lua"))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        found.sort();
        modules.extend(found);
    }
    modules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let cfg = Config::default();
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.adapter_hyprland);
        assert_eq!(cfg.dedup_window_ms, 250);
    }

    #[test]
    fn module_list_has_builtins() {
        let modules = modules();
        assert!(modules.iter().any(|name| name == "bread.monitors"));
    }
}
