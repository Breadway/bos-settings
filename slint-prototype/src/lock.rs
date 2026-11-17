//! breadlock/breadlock.toml, the lock screen appearance this app may edit.

use std::path::{Path, PathBuf};

use toml_edit::{value, DocumentMut, Item, Table};

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub background_mode: String,
    pub background_path: String,
    pub background_blur: bool,
    pub clock_format: String,
    pub font_family: String,
    pub fail_timeout_ms: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            background_mode: "color".into(),
            background_path: String::new(),
            background_blur: false,
            fail_timeout_ms: 800,
            clock_format: "%H:%M".into(),
            font_family: "Varela Round".into(),
        }
    }
}

pub fn path() -> PathBuf {
    config_dir().join("breadlock/breadlock.toml")
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

fn get_string(doc: &DocumentMut, table: &str, key: &str) -> Option<String> {
    doc.get(table)
        .and_then(Item::as_table)
        .and_then(|table| table.get(key))
        .and_then(Item::as_str)
        .map(str::to_string)
}

pub fn load_from(path: &Path) -> Result<Config, String> {
    let doc = read_doc(path)?;
    let mut cfg = Config::default();
    if let Some(mode) = get_string(&doc, "background", "mode") {
        cfg.background_mode = mode;
    }
    if let Some(format) = get_string(&doc, "clock", "format") {
        cfg.clock_format = format;
    }
    if let Some(path) = get_string(&doc, "background", "path") {
        cfg.background_path = path;
    }
    if let Some(blur) = doc
        .get("background")
        .and_then(Item::as_table)
        .and_then(|table| table.get("blur"))
        .and_then(Item::as_bool)
    {
        cfg.background_blur = blur;
    }
    if let Some(font) = get_string(&doc, "font", "family") {
        cfg.font_family = font;
    }
    if let Some(timeout) = doc
        .get("security")
        .and_then(Item::as_table)
        .and_then(|table| table.get("fail_timeout_ms"))
        .and_then(Item::as_integer)
    {
        cfg.fail_timeout_ms = timeout;
    }
    Ok(cfg)
}

pub fn load() -> Result<Config, String> {
    load_from(&path())
}

pub fn save(cfg: &Config) -> Result<(), String> {
    save_to(&path(), cfg)
}

pub fn save_to(path: &Path, cfg: &Config) -> Result<(), String> {
    let mut doc = read_doc(path)?;
    let mode = if cfg.background_mode == "image" {
        "image"
    } else {
        "color"
    };
    let mut background = table_with(
        doc.get("background").and_then(Item::as_table),
        "mode",
        value(mode),
    );
    background.insert("path", value(cfg.background_path.clone()));
    background.insert("blur", value(cfg.background_blur));
    let clock = table_with(
        doc.get("clock").and_then(Item::as_table),
        "format",
        value(cfg.clock_format.clone()),
    );
    let font = table_with(
        doc.get("font").and_then(Item::as_table),
        "family",
        value(cfg.font_family.clone()),
    );
    doc.insert("background", Item::Table(background));
    doc.insert("clock", Item::Table(clock));
    doc.insert("font", Item::Table(font));
    let security = table_with(
        doc.get("security").and_then(Item::as_table),
        "fail_timeout_ms",
        value(cfg.fail_timeout_ms),
    );
    doc.insert("security", Item::Table(security));
    write_atomic(path, &doc.to_string())
}

fn table_with(table: Option<&Table>, key: &str, item: Item) -> Table {
    let mut table = table.cloned().unwrap_or_default();
    table.insert(key, item);
    table
}

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!("{} has no parent", path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "breadlock.toml".into()),
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

pub fn clock_format(twelve_hour: bool, seconds: bool) -> &'static str {
    match (twelve_hour, seconds) {
        (false, false) => "%H:%M",
        (false, true) => "%H:%M:%S",
        (true, false) => "%I:%M %p",
        (true, true) => "%I:%M:%S %p",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-slint-lock-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("breadlock.toml")
    }

    #[test]
    fn reads_nested_keys_and_defaults() {
        let path = scratch("read");
        std::fs::write(
            &path,
            "[background]\nmode = \"image\"\npath = \"/tmp/x.png\"\n[clock]\nformat = \"%I:%M %p\"\n[font]\nfamily = \"Inter\"\n[input]\nfail_timeout_ms = 500\n",
        )
        .unwrap();
        let cfg = load_from(&path).unwrap();
        assert_eq!(cfg.background_mode, "image");
        assert_eq!(cfg.clock_format, "%I:%M %p");
        assert_eq!(cfg.font_family, "Inter");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn save_keeps_path_and_unmodelled_keys() {
        let path = scratch("save");
        std::fs::write(
            &path,
            "# keep\n[background]\nmode = \"color\"\npath = \"/tmp/wall.png\"\n[input]\nfail_timeout_ms = 900\n",
        )
        .unwrap();
        let cfg = Config {
            background_mode: "image".into(),
            clock_format: "%H:%M:%S".into(),
            font_family: "Outfit".into(),
        };
        save_to(&path, &cfg).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# keep"));
        assert!(text.contains("path = \"/tmp/wall.png\""));
        assert!(text.contains("fail_timeout_ms = 900"));
        assert!(text.contains("mode = \"image\""));
        assert!(text.contains("%H:%M:%S"));
        assert!(text.contains("family = \"Outfit\""));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
