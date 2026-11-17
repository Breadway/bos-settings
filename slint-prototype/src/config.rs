//! Non-destructive TOML editing shared by the pages that write bread
//! config files. Only the keys a page models are touched; comments,
//! formatting and unknown keys survive a save.

use std::path::{Path, PathBuf};

use toml_edit::{value, Array, DocumentMut, Item, Table, Value};

/// A missing file is an empty document. An unparseable one is backed up
/// once before that fallback, so a save cannot destroy it silently.
pub fn load_doc(path: &Path) -> DocumentMut {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    match text.parse::<DocumentMut>() {
        Ok(doc) => doc,
        Err(_) => {
            if !text.is_empty() {
                let backup = path.with_extension("toml.broken");
                let _ = std::fs::copy(path, backup);
            }
            DocumentMut::new()
        }
    }
}

pub fn save_doc(path: &Path, doc: &DocumentMut) -> Result<(), String> {
    atomic_write(path, &doc.to_string()).map_err(|error| error.to_string())
}

/// Temp file then rename, keeping one `.bak` of the previous contents.
pub fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let _ = std::fs::copy(path, path.with_extension("toml.bak"));
    }
    let temp = path.with_extension("toml.tmp");
    std::fs::write(&temp, contents)?;
    std::fs::rename(&temp, path)
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn get<'a>(doc: &'a DocumentMut, path: &[&str]) -> Option<&'a Item> {
    let mut table = doc.as_table();
    let (last, parents) = path.split_last()?;
    for key in parents {
        table = table.get(key)?.as_table()?;
    }
    table.get(last)
}

pub fn get_bool(doc: &DocumentMut, path: &[&str]) -> Option<bool> {
    get(doc, path)?.as_bool()
}

pub fn get_str(doc: &DocumentMut, path: &[&str]) -> Option<String> {
    get(doc, path)?.as_str().map(str::to_string)
}

pub fn get_i64(doc: &DocumentMut, path: &[&str]) -> Option<i64> {
    get(doc, path)?.as_integer()
}

pub fn get_f64(doc: &DocumentMut, path: &[&str]) -> Option<f64> {
    let item = get(doc, path)?;
    item.as_float()
        .or_else(|| item.as_integer().map(|number| number as f64))
}

pub fn get_str_list(doc: &DocumentMut, path: &[&str]) -> Vec<String> {
    match get(doc, path).and_then(Item::as_array) {
        Some(array) => array
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    }
}

/// Distinct from `get_str_list`: a missing key is `None`, an explicit
/// empty array is `Some(vec![])`, and anything else is `None`.
pub fn get_str_list_strict(doc: &DocumentMut, path: &[&str]) -> Option<Vec<String>> {
    get(doc, path)?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(str::to_string))
        .collect()
}

fn table_at_mut<'a>(doc: &'a mut DocumentMut, parents: &[&str]) -> &'a mut Table {
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
    table
}

fn set_item(doc: &mut DocumentMut, path: &[&str], item: Item) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    table_at_mut(doc, parents).insert(last, item);
}

pub fn set_bool(doc: &mut DocumentMut, path: &[&str], flag: bool) {
    set_item(doc, path, value(flag));
}

pub fn set_str(doc: &mut DocumentMut, path: &[&str], text: &str) {
    set_item(doc, path, value(text));
}

pub fn set_i64(doc: &mut DocumentMut, path: &[&str], number: i64) {
    set_item(doc, path, value(number));
}

pub fn set_f64(doc: &mut DocumentMut, path: &[&str], number: f64) {
    set_item(doc, path, value(number));
}

pub fn set_str_list(doc: &mut DocumentMut, path: &[&str], items: &[String]) {
    let mut array = Array::new();
    for item in items {
        array.push(item.as_str());
    }
    set_item(doc, path, Item::Value(Value::Array(array)));
}

pub fn set_str_or_remove(doc: &mut DocumentMut, path: &[&str], text: &str) {
    if text.is_empty() {
        remove(doc, path);
    } else {
        set_str(doc, path, text);
    }
}

pub fn remove(doc: &mut DocumentMut, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut table = doc.as_table_mut();
    for key in parents {
        match table.get_mut(key).and_then(Item::as_table_mut) {
            Some(next) => table = next,
            None => return,
        }
    }
    table.remove(last);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setters_keep_unknown_keys_and_comments() {
        let mut doc: DocumentMut = "# note\nkeep = 1\n\n[model]\npath = \"/x\"\n"
            .parse()
            .unwrap();
        set_bool(&mut doc, &["power", "enabled"], true);
        set_str_or_remove(&mut doc, &["model", "tokenizer"], "");
        let text = doc.to_string();
        assert!(text.contains("# note"));
        assert!(text.contains("keep = 1"));
        assert_eq!(get_bool(&doc, &["power", "enabled"]), Some(true));
        assert_eq!(get_str(&doc, &["model", "path"]).as_deref(), Some("/x"));
    }

    #[test]
    fn strict_lists_tell_missing_from_empty() {
        let doc: DocumentMut = "empty = []\n".parse().unwrap();
        assert_eq!(get_str_list_strict(&doc, &["empty"]), Some(Vec::new()));
        assert_eq!(get_str_list_strict(&doc, &["missing"]), None);
    }
}
