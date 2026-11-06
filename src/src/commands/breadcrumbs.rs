//! breadcrumbs.toml: Wi-Fi profile state machine. Schema mirrors
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
//! breadcrumbs.toml; leftover inline blocks from pre-split configs are
//! read once (only if `networks.toml` is missing) and migrated on the
//! next save. Other keys/comments in breadcrumbs.toml are preserved.
//!
//! The connect timeout is the file's `connect_wait` key. `nmcli_wait` is a
//! legacy alias breadcrumbs still accepts on read, but its serde mapping is
//! `alias = "nmcli_wait"` on the *same* field, so a file holding both fails
//! to parse with `duplicate field connect_wait`. This app therefore reads
//! either spelling but only ever writes the canonical `connect_wait`.

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
    /// means "keep whatever is already in networks.toml, or omit; NM
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
    /// Seconds to wait for a connect to reach ACTIVATED. Serialized to the
    /// webview as `nmcli_wait` (the key NetworkAdvanced.svelte binds); the
    /// on-disk key is breadcrumbs' canonical `connect_wait` (see
    /// `read_connect_wait` / `apply_settings`).
    #[serde(rename = "nmcli_wait")]
    connect_wait: i64,
    watch_interval: i64,
}

/// File key names for the connect timeout. `nmcli_wait` is read for
/// compatibility with configs written by older bos-settings releases, but
/// must never be written: breadcrumbs maps it as an alias of
/// `connect_wait`, and two such keys in one file are a parse error.
const CONNECT_WAIT_KEY: &str = "connect_wait";
const LEGACY_CONNECT_WAIT_KEY: &str = "nmcli_wait";
const DEFAULT_CONNECT_WAIT: i64 = 8;

fn read_connect_wait(doc: &DocumentMut) -> i64 {
    config::get_i64(doc, &["settings", CONNECT_WAIT_KEY])
        .or_else(|| config::get_i64(doc, &["settings", LEGACY_CONNECT_WAIT_KEY]))
        .unwrap_or(DEFAULT_CONNECT_WAIT)
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
    read_networks(&networks_source_doc(settings_doc, net_path))
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

fn save_networks(path: &Path, nets: &[Network], previous: &DocumentMut) -> Result<(), String> {
    let mut doc = networks_document(nets);
    carry_unmanaged_network_keys(&mut doc, previous);
    write_secure(path, &doc.to_string())
}

/// Keys this editor manages on a saved network. Everything else on the
/// entry (`dns`, the 802.1x `eap`/`identity`/`ca_cert`/`phase2`/
/// `client_cert`/`private_key`, and anything a newer breadcrumbs adds) is
/// carried over from the file as it was, matched by SSID. Without this a
/// save from this page silently deleted an enterprise network's login
/// settings and every per-network DNS override.
const MANAGED_NETWORK_KEYS: [&str; 3] = ["ssid", "password", "hidden"];

fn carry_unmanaged_network_keys(doc: &mut DocumentMut, previous: &DocumentMut) {
    let Some(old) = previous.get("networks").and_then(Item::as_array_of_tables) else {
        return;
    };
    let Some(new) = doc
        .get_mut("networks")
        .and_then(Item::as_array_of_tables_mut)
    else {
        return;
    };
    for table in new.iter_mut() {
        let ssid = table.get("ssid").and_then(Item::as_str).unwrap_or("");
        let Some(prev) = old
            .iter()
            .find(|t| t.get("ssid").and_then(Item::as_str) == Some(ssid))
        else {
            continue;
        };
        for (key, item) in prev.iter() {
            if !MANAGED_NETWORK_KEYS.contains(&key) && !table.contains_key(key) {
                table.insert(key, item.clone());
            }
        }
    }
}

/// The document saved networks are read from: `networks.toml` when it
/// exists, otherwise a legacy inline block in breadcrumbs.toml.
fn networks_source_doc(settings_doc: &DocumentMut, net_path: &Path) -> DocumentMut {
    if net_path.exists() {
        config::load_doc(net_path)
    } else {
        settings_doc.clone()
    }
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

/// Profile keys this editor manages. Others (`learn`, and anything a newer
/// breadcrumbs adds) are kept from the profile of the same name.
const MANAGED_PROFILE_KEYS: [&str; 6] = [
    "networks",
    "tailscale",
    "include_all_known",
    "detect_ssids",
    "bootstrap",
    "exit_node",
];

fn write_profiles(doc: &mut DocumentMut, profiles: &[Profile]) {
    let previous = doc.get("profiles").and_then(Item::as_table).cloned();
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
        if let Some(prev) = previous
            .as_ref()
            .and_then(|old| old.get(&p.name))
            .and_then(Item::as_table)
        {
            for (key, item) in prev.iter() {
                if !MANAGED_PROFILE_KEYS.contains(&key) {
                    t.insert(key, item.clone());
                }
            }
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
    config::set_i64(doc, &["settings", CONNECT_WAIT_KEY], settings.connect_wait);
    // Older builds wrote the legacy alias alongside (or instead of) the
    // canonical key; leaving it behind makes breadcrumbs reject the file.
    config::remove(doc, &["settings", LEGACY_CONNECT_WAIT_KEY]);
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
            connect_wait: read_connect_wait(&doc),
            watch_interval: config::get_i64(&doc, &["settings", "watch_interval"]).unwrap_or(12),
        },
        // Never ship a stored PSK to the webview; the password field is
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
    let previous_networks = networks_source_doc(&doc, net_path);
    let existing = read_networks(&previous_networks);
    apply_settings(&mut doc, &input.settings);
    write_profiles(&mut doc, &input.profiles);
    // Completes the pre-split migration: leftover [[networks]] must not
    // survive a save, even if the user only edited settings/profiles.
    doc.as_table_mut().remove("networks");
    config::save_doc(settings_path, &doc).map_err(|e| e.to_string())?;

    let merged = merge_network_passwords(input.networks, &existing);
    save_networks(net_path, &merged, &previous_networks)
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

/// The place breadcrumbs is using now (`breadcrumbs profile get`), or
/// `None` when breadcrumbs isn't installed.
#[tauri::command]
pub async fn get_current_place() -> Option<String> {
    let out = tokio::process::Command::new("breadcrumbs")
        .args(["profile", "get"])
        .output()
        .await
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !name.is_empty()).then_some(name)
}

/// Switches to a saved place and lets breadcrumbs join its networks
/// (`breadcrumbs profile set <name>`).
#[tauri::command]
pub async fn set_current_place(name: String) -> Result<(), String> {
    let cfg = get_breadcrumbs_config();
    if !cfg.profiles.iter().any(|p| p.name == name) {
        return Err(format!("There is no place called {name}"));
    }
    let out = tokio::process::Command::new("breadcrumbs")
        .args(["profile", "set", &name])
        .output()
        .await
        .map_err(|e| format!("Couldn't run breadcrumbs: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(super::util::fail_output(&out, "Switching place"))
    }
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
            connect_wait: 8,
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
    fn save_keeps_enterprise_and_dns_keys_and_profile_learn() {
        let dir = tmp_dir("carry");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(
            &settings,
            "[settings]\ndns = \"1.1.1.1\"\n\n[profiles.School]\nnetworks = [\"Uni\"]\ntailscale = false\ninclude_all_known = false\nlearn = true\n",
        )
        .unwrap();
        std::fs::write(
            &nets,
            "[[networks]]\nssid = \"Uni\"\npassword = \"pw\"\neap = \"peap\"\nidentity = \"me@uni\"\ndns = \"9.9.9.9\"\nhidden = false\n",
        )
        .unwrap();
        let cfg = load_from(&settings, &nets);
        save_to(
            &settings,
            &nets,
            SaveBreadcrumbsInput {
                settings: sample_settings(),
                networks: cfg.networks.clone(),
                profiles: cfg.profiles.clone(),
            },
        )
        .unwrap();
        let nets_text = std::fs::read_to_string(&nets).unwrap();
        for kept in [
            "eap = \"peap\"",
            "identity = \"me@uni\"",
            "dns = \"9.9.9.9\"",
            "pw",
        ] {
            assert!(nets_text.contains(kept), "lost {kept}: {nets_text}");
        }
        let settings_text = std::fs::read_to_string(&settings).unwrap();
        assert!(settings_text.contains("learn = true"), "{settings_text}");
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

    /// A breadcrumbs-generated file: a comment, the canonical
    /// `connect_wait = 8` breadcrumbs serializes itself, and settings keys
    /// this editor does not own.
    fn breadcrumbs_generated_config() -> &'static str {
        "# written by breadcrumbs\n\
         [settings]\n\
         default_profile = \"away\"\n\
         connect_wait = 8\n\
         dns = \"1.1.1.1\"\n\
         watch_interval = 12\n\
         interface = \"wlan0\"\n\
         exit_nodes = [\"alpha\"]\n\
         \n\
         [profiles.away]\n\
         networks = []\n\
         tailscale = false\n\
         include_all_known = true\n\
         learn = true\n"
    }

    /// Round-trip exactly as the UI does: read the file, then save the values
    /// that were read straight back.
    fn load_and_save(settings_path: &Path, net_path: &Path) {
        let BreadcrumbsConfig {
            settings,
            networks,
            profiles,
        } = load_from(settings_path, net_path);
        save_to(
            settings_path,
            net_path,
            SaveBreadcrumbsInput {
                settings,
                networks,
                profiles,
            },
        )
        .unwrap();
    }

    /// Count top-level assignments to `key` (indentation allowed). A file
    /// must contain zero or one, never both spellings at once.
    fn count_key(text: &str, key: &str) -> usize {
        text.lines()
            .filter(|line| {
                line.trim_start()
                    .strip_prefix(key)
                    .is_some_and(|rest| rest.trim_start().starts_with('='))
            })
            .count()
    }

    /// Minimal mirror of breadcrumbs' `Settings` serde rule for the field
    /// this app writes: canonical `connect_wait`, legacy alias `nmcli_wait`,
    /// same field. Serde rejects a file holding both with
    /// `duplicate field connect_wait`, which is the corruption this module
    /// must never recreate. toml_edit is already a dependency, so only its
    /// `serde` feature is needed to parse with the real rule.
    #[derive(Deserialize, Default)]
    struct BreadcrumbsSettingsRules {
        #[serde(default = "default_connect_wait", alias = "nmcli_wait")]
        connect_wait: u32,
    }

    #[derive(Deserialize, Default)]
    struct BreadcrumbsConfigRules {
        #[serde(default)]
        settings: BreadcrumbsSettingsRules,
    }

    fn default_connect_wait() -> u32 {
        8
    }

    fn breadcrumbs_connect_wait(text: &str) -> u32 {
        toml_edit::de::from_str::<BreadcrumbsConfigRules>(text)
            .unwrap_or_else(|e| panic!("breadcrumbs would reject this config: {e}\n{text}"))
            .settings
            .connect_wait
    }

    #[test]
    fn save_over_breadcrumbs_generated_config_keeps_one_canonical_key() {
        let dir = tmp_dir("canonical");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(&settings, breadcrumbs_generated_config()).unwrap();

        let cfg = load_from(&settings, &nets);
        assert_eq!(cfg.settings.connect_wait, 8);

        load_and_save(&settings, &nets);

        let text = std::fs::read_to_string(&settings).unwrap();
        assert_eq!(count_key(&text, "connect_wait"), 1, "{text}");
        assert_eq!(count_key(&text, "nmcli_wait"), 0, "{text}");
        assert!(text.contains("connect_wait = 8"), "{text}");
        assert_eq!(breadcrumbs_connect_wait(&text), 8, "{text}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_nmcli_wait_loads_and_migrates_to_connect_wait() {
        let dir = tmp_dir("migrate-alias");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(
            &settings,
            "# written by an older bos-settings\n[settings]\nnmcli_wait = 5\n",
        )
        .unwrap();

        let cfg = load_from(&settings, &nets);
        assert_eq!(cfg.settings.connect_wait, 5, "legacy alias must be read");

        load_and_save(&settings, &nets);

        let text = std::fs::read_to_string(&settings).unwrap();
        assert_eq!(count_key(&text, "connect_wait"), 1, "{text}");
        assert_eq!(count_key(&text, "nmcli_wait"), 0, "{text}");
        assert!(text.contains("connect_wait = 5"), "{text}");
        assert!(
            text.contains("# written by an older bos-settings"),
            "{text}"
        );
        assert_eq!(breadcrumbs_connect_wait(&text), 5, "{text}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_with_both_keys_loads_and_saves_to_canonical_only() {
        let duplicate = "[settings]\nconnect_wait = 6\nnmcli_wait = 9\n";
        assert!(
            toml_edit::de::from_str::<BreadcrumbsConfigRules>(duplicate).is_err(),
            "fixture must reproduce breadcrumbs' duplicate-field rejection"
        );

        let dir = tmp_dir("both-keys");
        let settings = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(&settings, format!("# both\n{duplicate}")).unwrap();

        let cfg = load_from(&settings, &nets);
        assert_eq!(
            cfg.settings.connect_wait, 6,
            "canonical key wins when a buggy file has both"
        );

        load_and_save(&settings, &nets);

        let text = std::fs::read_to_string(&settings).unwrap();
        assert_eq!(count_key(&text, "connect_wait"), 1, "{text}");
        assert_eq!(count_key(&text, "nmcli_wait"), 0, "{text}");
        assert!(text.contains("connect_wait = 6"), "{text}");
        assert_eq!(breadcrumbs_connect_wait(&text), 6, "{text}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_round_trips_unmanaged_keys_and_comments() {
        let dir = tmp_dir("round-trip");
        let cfg_path = dir.join("breadcrumbs.toml");
        let nets = dir.join("networks.toml");
        std::fs::write(
            &cfg_path,
            "# top of file\n\
             [settings]\n\
             dns = \"9.9.9.9\"\n\
             # physical radio choice\n\
             interface = \"wlan1\"\n\
             exit_nodes = [\"alpha\", \"beta\"]\n\
             connect_wait = 8\n\
             \n\
             [profiles.home]\n\
             networks = [\"HomeNet\"]\n\
             tailscale = true\n\
             learn = true\n",
        )
        .unwrap();

        let BreadcrumbsConfig {
            mut settings,
            networks,
            profiles,
        } = load_from(&cfg_path, &nets);
        settings.connect_wait = 30;
        save_to(
            &cfg_path,
            &nets,
            SaveBreadcrumbsInput {
                settings,
                networks,
                profiles,
            },
        )
        .unwrap();

        let text = std::fs::read_to_string(&cfg_path).unwrap();
        for kept in [
            "# top of file",
            "dns = \"9.9.9.9\"",
            "# physical radio choice",
            "interface = \"wlan1\"",
            "exit_nodes = [\"alpha\", \"beta\"]",
            "learn = true",
        ] {
            assert!(text.contains(kept), "lost {kept}: {text}");
        }
        assert_eq!(count_key(&text, "connect_wait"), 1, "{text}");
        assert_eq!(count_key(&text, "nmcli_wait"), 0, "{text}");
        assert!(text.contains("connect_wait = 30"), "{text}");
        assert_eq!(breadcrumbs_connect_wait(&text), 30, "{text}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
