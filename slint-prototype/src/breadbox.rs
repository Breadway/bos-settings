//! breadbox/config.toml: apps and categories the launcher lists first.

use std::path::PathBuf;

use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use crate::config;

#[derive(Clone)]
pub struct Context {
    pub name: String,
    pub priority: Vec<String>,
}

pub fn path() -> PathBuf {
    config::config_dir().join("breadbox/config.toml")
}

fn read_contexts(doc: &DocumentMut) -> Vec<Context> {
    let Some(aot) = doc.get("context").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    aot.iter()
        .map(|table| Context {
            name: table
                .get("name")
                .and_then(Item::as_str)
                .unwrap_or("")
                .to_string(),
            priority: table
                .get("priority")
                .and_then(Item::as_array)
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|value| value.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

pub fn load() -> Vec<Context> {
    read_contexts(&config::load_doc(&path()))
}

pub fn save(contexts: &[Context]) -> Result<(), String> {
    let path = path();
    let mut doc = config::load_doc(&path);
    let mut aot = ArrayOfTables::new();
    for ctx in contexts {
        let mut table = Table::new();
        table.insert("name", value(&ctx.name));
        let mut priority = Array::new();
        for item in &ctx.priority {
            priority.push(item.as_str());
        }
        table.insert("priority", value(priority));
        aot.push(table);
    }
    doc.as_table_mut()
        .insert("context", Item::ArrayOfTables(aot));
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
    fn missing_file_has_no_contexts() {
        let doc = config::load_doc(&PathBuf::from("/nonexistent/breadbox.toml"));
        assert!(read_contexts(&doc).is_empty());
    }

    #[test]
    fn reads_singular_context_key() {
        let doc: DocumentMut = "[[context]]\nname = \"default\"\npriority = [\"Zen Browser\"]\n"
            .parse()
            .unwrap();
        let contexts = read_contexts(&doc);
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].priority, vec!["Zen Browser"]);
    }
}
