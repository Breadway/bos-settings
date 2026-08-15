//! breadcrumbs.toml — Wi-Fi profile state machine. Schema mirrors
//! breadcrumbs/src/config.rs:
//!   [settings]              scalar tunables  (this file)
//!   [profiles.<name>]       per-location profile (networks, tailscale, …)
//!
//! Saved networks (SSID + optional local password) live in a *separate*
//! `networks.toml` (0600) next to breadcrumbs.toml. breadcrumbs v2 stores
//! them there so a file people hand-edit / dotfile does not also carry
//! plaintext Wi-Fi credentials. After the first successful connect,
//! breadcrumbs clears the local password and NetworkManager owns the
//! secret; `None` means "NM already has it" or "open network".
//!
//! `[settings]` is edited in place via toml_edit; `profiles` are rewritten
//! from their editor on save. `[[networks]]` is never written back into
//! breadcrumbs.toml — leftover inline blocks from pre-split configs are
//! read once (only if `networks.toml` is missing) and migrated on the
//! next save. Other keys/comments in breadcrumbs.toml are preserved.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

use super::config;

fn settings_path() -> PathBuf {
    config::config_dir().join("breadcrumbs/breadcrumbs.toml")
}

fn networks_path() -> PathBuf {
    config::config_dir().join("breadcrumbs/networks.toml")
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Network {
    ssid: String,
    /// Write-only from the UI's point of view. `get_breadcrumbs_config`
    /// never returns a stored PSK (always `None`). On save, `None` / empty
    /// means "keep whatever is already in networks.toml, or omit — NM
    /// remembers." A non-empty value is written only to networks.toml.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(default)]
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
        .map(|t| {
            let password = t
                .get("password")
                .and_then(Item::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            Network {
                ssid: t
                    .get("ssid")
                    .and_then(Item::as_str)
                    .unwrap_or("")
                    .to_string(),
                password,
                hidden: t.get("hidden").and_then(Item::as_bool).unwrap_or(false),
            }
        })
        .collect()
}

fn networks_document(nets: &[Network]) -> DocumentMut {
    let mut doc = DocumentMut::new();
    let mut aot = ArrayOfTables::new();
    for n in nets {
        if n.ssid.trim().is_empty() {
            continue;
        }
        let mut t = Table::new();
        t.insert("ssid", value(&n.ssid));
        if let Some(pw) = n
            .password
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            t.insert("password", value(pw));
        }
        t.insert("hidden", value(n.hidden));
        aot.push(t);
    }
    doc.as_table_mut()
        .insert("networks", Item::ArrayOfTables(aot));
    doc
}

/// Load saved networks. `networks.toml` wins when present; otherwise fall
/// back to a leftover inline `[[networks]]` block in breadcrumbs.toml so a
/// pre-split config still shows up until the next save migrates it.
fn load_networks(settings_doc: &DocumentMut, net_path: &Path) -> Vec<Network> {
    if net_path.exists() {
        let doc = config::load_doc(net_path);
        return read_networks(&doc);
    }
    read_networks(settings_doc)
}

fn redact_passwords(nets: Vec<Network>) -> Vec<Network> {
    nets.into_iter()
        .map(|n| Network {
            password: None,
            ..n
        })
        .collect()
}

fn incoming_password(n: &Network) -> Option<String> {
    n.password
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Empty / omitted password from the UI means "keep the on-disk secret for
/// this SSID (if any), otherwise let NetworkManager remember." A typed
/// value replaces it. Matching is by SSID; a renamed SSID is a new
/// network and does not inherit the old password.
fn merge_network_passwords(incoming: Vec<Network>, existing: &[Network]) -> Vec<Network> {
    incoming
        .into_iter()
        .filter(|n| !n.ssid.trim().is_empty())
        .map(|mut n| {
            n.password = incoming_password(&n).or_else(|| {
                existing
                    .iter()
                    .find(|e| e.ssid == n.ssid)
                    .and_then(|e| e.password.clone())
            });
            n
        })
        .collect()
}

/// Atomic write with mode 0600 set on the temp file *before* any bytes
/// land, so a secrets file is never briefly world-readable. Also re-applies
/// 0600 on the destination in case an older world-readable networks.toml
/// was being replaced (rename keeps the new inode's mode).
fn write_secure(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }
    bread_utils::atomic::write_atomic(path, contents, Some(0o600))
        .map_err(|e| format!("writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn save_networks(path: &Path, nets: &[Network]) -> Result<(), String> {
    write_secure(path, &networks_document(nets).to_string())
}

fn read_profiles(doc: &DocumentMut) -> Vec<Profile> {
    let Some(tbl) = doc.get("profiles").and_then(Item::as_table) else {
        return Vec::new();
    };
    let str_list = |item: Option<&Item>| -> Vec<String> {
        item.and_then(Item::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    tbl.iter()
        .filter_map(|(name, item)| {
            let p = item.as_table()?;
            Some(Profile {
                name: name.to_string(),
                networks: str_list(p.get("networks")),
                detect_ssids: str_list(p.get("detect_ssids")),
                bootstrap: p
                    .get("bootstrap")
                    .and_then(Item::as_str)
                    .unwrap_or("")
                    .to_string(),
                exit_node: p
                    .get("exit_node")
                    .and_then(Item::as_str)
                    .unwrap_or("")
                    .to_string(),
                tailscale: p.get("tailscale").and_then(Item::as_bool).unwrap_or(false),
                include_all_known: p
                    .get("include_all_known")
                    .and_then(Item::as_bool)
                    .unwrap_or(false),
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

fn apply_settings(doc: &mut DocumentMut, settings: &Settings) {
    config::set_str(
        doc,
        &["settings", "default_profile"],
        &settings.default_profile,
    );
    config::set_str(doc, &["settings", "dns"], &settings.dns);
    config::set_str_or_remove(doc, &["settings", "exit_node"], &settings.exit_node);
    config::set_str(doc, &["settings", "ping_host"], &settings.ping_host);
    config::set_str(
        doc,
        &["settings", "connectivity_url"],
        &settings.connectivity_url,
    );
    config::set_i64(doc, &["settings", "nmcli_wait"], settings.nmcli_wait);
    config::set_i64(
        doc,
        &["settings", "watch_interval"],
        settings.watch_interval,
    );
}

fn load_from(settings_path: &Path, net_path: &Path) -> BreadcrumbsConfig {
    let doc = config::load_doc(settings_path);
    BreadcrumbsConfig {
        settings: Settings {
            // breadcrumbs' own default_profile_name() is "away", not "home".
            default_profile: config::get_str(&doc, &["settings", "default_profile"])
                .unwrap_or_else(|| "away".into()),
            dns: config::get_str(&doc, &["settings", "dns"]).unwrap_or_else(|| "1.1.1.1".into()),
            exit_node: config::get_str(&doc, &["settings", "exit_node"]).unwrap_or_default(),
            ping_host: config::get_str(&doc, &["settings", "ping_host"])
                .unwrap_or_else(|| "1.1.1.1".into()),
            connectivity_url: config::get_str(&doc, &["settings", "connectivity_url"])
                .unwrap_or_else(|| "http://connectivitycheck.gstatic.com/generate_204".into()),
            nmcli_wait: config::get_i64(&doc, &["settings", "nmcli_wait"]).unwrap_or(8),
            watch_interval: config::get_i64(&doc, &["settings", "watch_interval"]).unwrap_or(12),
        },
        // Never ship a stored PSK to the webview — the password field is
        // write-only (empty = keep existing / let NM remember).
        networks: redact_passwords(load_networks(&doc, net_path)),
        profiles: read_profiles(&doc),
    }
}

fn save_to(
    settings_path: &Path,
    net_path: &Path,
    input: SaveBreadcrumbsInput,
) -> Result<(), String> {
    let mut doc = config::load_doc(settings_path);
    let existing = load_networks(&doc, net_path);
    apply_settings(&mut doc, &input.settings);
    write_profiles(&mut doc, &input.profiles);
    // Completes the pre-split migration: leftover [[networks]] must not
    // survive a save, even if the user only edited settings/profiles.
    doc.as_table_mut().remove("networks");
    config::save_doc(settings_path, &doc).map_err(|e| e.to_string())?;

    let merged = merge_network_passwords(input.networks, &existing);
    save_networks(net_path, &merged)
}

#[tauri::command]
pub fn get_breadcrumbs_config() -> BreadcrumbsConfig {
    load_from(&settings_path(), &networks_path())
}

#[derive(Deserialize)]
pub struct SaveBreadcrumbsInput {
    settings: Settings,
    networks: Vec<Network>,
    profiles: Vec<Profile>,
}

#[tauri::command]
pub fn save_breadcrumbs_config(input: SaveBreadcrumbsInput) -> Result<(), String> {
    save_to(&settings_path(), &networks_path(), input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-breadcrumbs-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_settings() -> Settings {
        Settings {
            default_profile: "away".into(),
            dns: "1.1.1.1".into(),
            exit_node: String::new(),
            ping_host: "1.1.1.1".into(),
            connectivity_url: "http://connectivitycheck.gstatic.com/generate_204".into(),
            nmcli_wait: 8,
            watch_interval: 12,
        }
    }

    #[test]
    fn networks_document_omits_password_when_none() {
        let nets = vec![Network {
            ssid: "Cafe".into(),
            password: None,
            hidden: false,
        }];
        let text = networks_document(&nets).to_string();
        assert!(text.contains("ssid"));
        assert!(!text.contains("password"), "text: {text}");
    }

    #[test]
    fn networks_document_writes_password_when_present() {
        let nets = vec![Network {
            ssid: "Cafe".into(),
            password: Some("hunter2".into()),
            hidden: true,
        }];
        let text = networks_document(&nets).to_string();
        assert!(text.contains("hunter2"));
        assert!(text.contains("hidden = true"));
        let back = read_networks(&text.parse().unwrap());
        assert_eq!(back[0].password.as_deref(), Some("hunter2"));
        assert!(back[0].hidden);
    }

    #[test]
    fn merge_keeps_existing_password_when_incoming_empty() {
        let existing = vec![Network {
            ssid: "Cafe".into(),
            password: Some("hunter2".into()),
            hidden: false,
        }];
        let incoming = vec![Network {
            ssid: "Cafe".into(),
            password: Some(String::new()),
            hidden: true,
        }];
        let merged = merge_network_passwords(incoming, &existing);
        assert_eq!(merged[0].password.as_deref(), Some("hunter2"));
        assert!(merged[0].hidden);
    }

    #[test]
    fn merge_replaces_password_when_incoming_set() {
        let existing = vec![Network {
            ssid: "Cafe".into(),
            password: Some("old".into()),
            hidden: false,
        }];
        let incoming = vec![Network {
            ssid: "Cafe".into(),
            password: Some("new".into()),
            hidden: false,
        }];
        let merged = merge_network_passwords(incoming, &existing);
        assert_eq!(merged[0].password.as_deref(), Some("new"));
    }

    #[test]
    fn get_never_returns_stored_password() {
        let dir = tmp_dir("redact");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(&settings, "[settings]\ndns = \"9.9.9.9\"\n").unwrap();
        std::fs::write(
            &nets,
            "[[networks]]\nssid = \"Cafe\"\npassword = \"hunter2\"\nhidden = false\n",
        )
        .unwrap();

        let cfg = load_from(&settings, &nets);
        assert_eq!(cfg.settings.dns, "9.9.9.9");
        assert_eq!(cfg.networks.len(), 1);
        assert_eq!(cfg.networks[0].ssid, "Cafe");
        assert_eq!(cfg.networks[0].password, None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_writes_networks_toml_not_inline_and_uses_0600() {
        let dir = tmp_dir("split");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(
            &settings,
            "# keep me\n[settings]\ndns = \"1.1.1.1\"\n\n[[networks]]\nssid = \"Old\"\npassword = \"legacy\"\n",
        )
        .unwrap();

        save_to(
            &settings,
            &nets,
            SaveBreadcrumbsInput {
                settings: sample_settings(),
                networks: vec![Network {
                    ssid: "Cafe".into(),
                    password: Some("hunter2".into()),
                    hidden: false,
                }],
                profiles: vec![],
            },
        )
        .unwrap();

        let settings_text = std::fs::read_to_string(&settings).unwrap();
        assert!(
            settings_text.contains("# keep me"),
            "toml_edit must keep comments"
        );
        assert!(
            !settings_text.contains("[[networks]]"),
            "inline networks must be gone"
        );
        assert!(
            !settings_text.contains("hunter2"),
            "PSK must not land in breadcrumbs.toml"
        );
        assert!(!settings_text.contains("legacy"));

        let nets_text = std::fs::read_to_string(&nets).unwrap();
        assert!(nets_text.contains("Cafe"));
        assert!(nets_text.contains("hunter2"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&nets).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "networks.toml must be 0600, got {mode:o}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_with_empty_password_keeps_existing_secret() {
        let dir = tmp_dir("keep-secret");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(&settings, "[settings]\ndns = \"1.1.1.1\"\n").unwrap();
        std::fs::write(
            &nets,
            "[[networks]]\nssid = \"Cafe\"\npassword = \"hunter2\"\nhidden = false\n",
        )
        .unwrap();

        save_to(
            &settings,
            &nets,
            SaveBreadcrumbsInput {
                settings: sample_settings(),
                networks: vec![Network {
                    ssid: "Cafe".into(),
                    password: None,
                    hidden: true,
                }],
                profiles: vec![],
            },
        )
        .unwrap();

        let nets_text = std::fs::read_to_string(&nets).unwrap();
        assert!(
            nets_text.contains("hunter2"),
            "empty password must keep existing secret"
        );
        assert!(nets_text.contains("hidden = true"));
        assert!(!std::fs::read_to_string(&settings)
            .unwrap()
            .contains("hunter2"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_inline_networks_load_when_networks_toml_missing() {
        let dir = tmp_dir("legacy");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(
            &settings,
            "[settings]\ndns = \"8.8.8.8\"\n\n[[networks]]\nssid = \"LegacyNet\"\npassword = \"secret\"\n",
        )
        .unwrap();

        let cfg = load_from(&settings, &nets);
        assert_eq!(cfg.networks.len(), 1);
        assert_eq!(cfg.networks[0].ssid, "LegacyNet");
        assert_eq!(cfg.networks[0].password, None, "still redacted to the UI");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
