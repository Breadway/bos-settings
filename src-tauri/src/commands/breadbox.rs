//! breadbox config.toml — launcher contexts. Schema mirrors breadbox-shared
//! (`#[serde(rename = "context")]` — the TOML key is `[[context]]`,
//! singular, despite the Rust field being `contexts`), with `name` +
//! `priority`, an ordered list of app/category hints. The context array is
//! rewritten on save; any other top-level keys/comments are preserved.

use serde::{Deserialize, Serialize};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadbox/config.toml")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Context {
    name: String,
    priority: Vec<String>,
}

fn read_contexts(doc: &DocumentMut) -> Vec<Context> {
    let Some(aot) = doc.get("context").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    aot.iter()
        .map(|t| Context {
            name: t.get("name").and_then(Item::as_str).unwrap_or("").to_string(),
            priority: t
                .get("priority")
                .and_then(Item::as_array)
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        })
        .collect()
}

fn write_contexts(doc: &mut DocumentMut, ctxs: &[Context]) {
    let mut aot = ArrayOfTables::new();
    for ctx in ctxs {
        let mut t = Table::new();
        t.insert("name", value(&ctx.name));
        let mut arr = Array::new();
        for p in &ctx.priority {
            arr.push(p.as_str());
        }
        t.insert("priority", value(arr));
        aot.push(t);
    }
    doc.as_table_mut().insert("context", Item::ArrayOfTables(aot));
}

#[tauri::command]
pub fn get_breadbox_contexts() -> Vec<Context> {
    read_contexts(&config::load_doc(&config_path()))
}

#[tauri::command]
pub fn save_breadbox_contexts(contexts: Vec<Context>) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    write_contexts(&mut doc, &contexts);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}
