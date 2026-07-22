//! breadcrumbs.toml — Wi-Fi profile state machine. Schema mirrors
//! breadcrumbs/src/config.rs:
//!   [settings]              scalar tunables
//!   [[networks]]            saved networks (ssid / password / hidden)
//!   [profiles.<name>]       per-location profile (networks, tailscale, …)
//! `[settings]` is edited in place; `networks`/`profiles` are rewritten from
//! their editors on save. Other keys/comments are preserved.

use serde::{Deserialize, Serialize};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use super::config;

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("breadcrumbs/breadcrumbs.toml")
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Network {
    ssid: String,
    password: String,
    hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Profile {
    name: String,
    networks: Vec<String>,
    detect_ssids: Vec<String>,
    bootstrap: String,
    exit_node: String,
    tailscale: bool,
    include_all_known: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    default_profile: String,
    dns: String,
    exit_node: String,
    ping_host: String,
    connectivity_url: String,
    nmcli_wait: i64,
    watch_interval: i64,
}

#[derive(Serialize)]
pub struct BreadcrumbsConfig {
    settings: Settings,
    networks: Vec<Network>,
    profiles: Vec<Profile>,
}

fn read_networks(doc: &DocumentMut) -> Vec<Network> {
    let Some(aot) = doc.get("networks").and_then(Item::as_array_of_tables) else {
        return Vec::new();
    };
    aot.iter()
        .map(|t| Network {
            ssid: t.get("ssid").and_then(Item::as_str).unwrap_or("").to_string(),
            password: t.get("password").and_then(Item::as_str).unwrap_or("").to_string(),
            hidden: t.get("hidden").and_then(Item::as_bool).unwrap_or(false),
        })
        .collect()
}

fn write_networks(doc: &mut DocumentMut, nets: &[Network]) {
    let mut aot = ArrayOfTables::new();
    for n in nets {
        let mut t = Table::new();
        t.insert("ssid", value(&n.ssid));
        t.insert("password", value(&n.password));
        t.insert("hidden", value(n.hidden));
        aot.push(t);
    }
    doc.as_table_mut().insert("networks", Item::ArrayOfTables(aot));
}

fn read_profiles(doc: &DocumentMut) -> Vec<Profile> {
    let Some(tbl) = doc.get("profiles").and_then(Item::as_table) else {
        return Vec::new();
    };
    let str_list = |item: Option<&Item>| -> Vec<String> {
        item.and_then(Item::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    tbl.iter()
        .filter_map(|(name, item)| {
            let p = item.as_table()?;
            Some(Profile {
                name: name.to_string(),
                networks: str_list(p.get("networks")),
                detect_ssids: str_list(p.get("detect_ssids")),
                bootstrap: p.get("bootstrap").and_then(Item::as_str).unwrap_or("").to_string(),
                exit_node: p.get("exit_node").and_then(Item::as_str).unwrap_or("").to_string(),
                tailscale: p.get("tailscale").and_then(Item::as_bool).unwrap_or(false),
                include_all_known: p.get("include_all_known").and_then(Item::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn write_profiles(doc: &mut DocumentMut, profiles: &[Profile]) {
    let mut tbl = Table::new();
    let to_arr = |items: &[String]| {
        let mut a = Array::new();
        for s in items {
            a.push(s.as_str());
        }
        a
    };
    for p in profiles {
        if p.name.is_empty() {
            continue;
        }
        let mut t = Table::new();
        t.insert("networks", value(to_arr(&p.networks)));
        t.insert("tailscale", value(p.tailscale));
        t.insert("include_all_known", value(p.include_all_known));
        if !p.detect_ssids.is_empty() {
            t.insert("detect_ssids", value(to_arr(&p.detect_ssids)));
        }
        if !p.bootstrap.is_empty() {
            t.insert("bootstrap", value(&p.bootstrap));
        }
        if !p.exit_node.is_empty() {
            t.insert("exit_node", value(&p.exit_node));
        }
        tbl.insert(&p.name, Item::Table(t));
    }
    doc.as_table_mut().insert("profiles", Item::Table(tbl));
}

#[tauri::command]
pub fn get_breadcrumbs_config() -> BreadcrumbsConfig {
    let doc = config::load_doc(&config_path());
    BreadcrumbsConfig {
        settings: Settings {
            // breadcrumbs' own default_profile_name() is "away", not "home".
            default_profile: config::get_str(&doc, &["settings", "default_profile"]).unwrap_or_else(|| "away".into()),
            dns: config::get_str(&doc, &["settings", "dns"]).unwrap_or_else(|| "1.1.1.1".into()),
            exit_node: config::get_str(&doc, &["settings", "exit_node"]).unwrap_or_default(),
            ping_host: config::get_str(&doc, &["settings", "ping_host"]).unwrap_or_else(|| "1.1.1.1".into()),
            connectivity_url: config::get_str(&doc, &["settings", "connectivity_url"])
                .unwrap_or_else(|| "http://connectivitycheck.gstatic.com/generate_204".into()),
            nmcli_wait: config::get_i64(&doc, &["settings", "nmcli_wait"]).unwrap_or(8),
            watch_interval: config::get_i64(&doc, &["settings", "watch_interval"]).unwrap_or(12),
        },
        networks: read_networks(&doc),
        profiles: read_profiles(&doc),
    }
}

#[derive(Deserialize)]
pub struct SaveBreadcrumbsInput {
    settings: Settings,
    networks: Vec<Network>,
    profiles: Vec<Profile>,
}

#[tauri::command]
pub fn save_breadcrumbs_config(input: SaveBreadcrumbsInput) -> Result<(), String> {
    let path = config_path();
    let mut doc = config::load_doc(&path);
    config::set_str(&mut doc, &["settings", "default_profile"], &input.settings.default_profile);
    config::set_str(&mut doc, &["settings", "dns"], &input.settings.dns);
    config::set_str_or_remove(&mut doc, &["settings", "exit_node"], &input.settings.exit_node);
    config::set_str(&mut doc, &["settings", "ping_host"], &input.settings.ping_host);
    config::set_str(&mut doc, &["settings", "connectivity_url"], &input.settings.connectivity_url);
    config::set_i64(&mut doc, &["settings", "nmcli_wait"], input.settings.nmcli_wait);
    config::set_i64(&mut doc, &["settings", "watch_interval"], input.settings.watch_interval);
    write_networks(&mut doc, &input.networks);
    write_profiles(&mut doc, &input.profiles);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}
