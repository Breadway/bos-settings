//! The "Bug reports" page: breadbug's switches, history and local data.
//!
//! breadbug itself (the `breadbug` binary and breadd's store) lives in the
//! bread repo; this page only edits `~/.config/bread/breadbug.toml`
//! non-destructively and asks the daemon to apply it, and reads/clears the
//! files breadbug keeps. Nothing here ever sends a report.

use std::path::PathBuf;

use bread_bug::config::{store_root, StoreLimits, DEFAULT_ENDPOINT};
use bread_bug::consent::CONSENT_TEXT;
use bread_bug::store::{valid_app_id, Reader};
use bread_bug::system::installed_apps;
use bread_bug::upload::{state_dir, Client, History, HistoryKind, Queue};
use serde::Serialize;

use super::config::{
    get_bool, get_i64, get_str, get_str_list, load_doc, save_doc, set_bool, set_i64, set_str_list,
};

fn breadbug_toml() -> PathBuf {
    bread_utils::xdg::config_home()
        .join("bread")
        .join("breadbug.toml")
}

#[derive(Serialize)]
pub struct AppChoice {
    pub id: String,
    pub installed: bool,
    pub excluded: bool,
}

#[derive(Serialize)]
pub struct BreadbugSettings {
    pub breadbug_installed: bool,
    pub upload_enabled: bool,
    pub notify_fixed: bool,
    pub endpoint: String,
    pub apps: Vec<AppChoice>,
    pub store_bytes: u64,
    /// Current total store cap in bytes (`store.max_total_bytes`).
    pub store_cap_bytes: u64,
    /// The default cap, so the page can say what "default" means.
    pub store_cap_default_bytes: u64,
    pub queued: usize,
    pub consent_text: String,
}

// A cap outside this range is not a useful local bound: under 1 MiB breadbug
// could not hold a single report's logs, and above 1 GiB it competes with the
// user's actual work. The config file itself accepts any value; this is the
// settings page's guard against a typo.
pub const STORE_CAP_MIN: u64 = 1 << 20;
pub const STORE_CAP_MAX: u64 = 1 << 30;

/// Validate a requested store cap and return it in bytes.
pub fn clean_store_cap(bytes: u64) -> Result<u64, String> {
    if !(STORE_CAP_MIN..=STORE_CAP_MAX).contains(&bytes) {
        return Err(format!(
            "The storage cap must be between {} and {} MiB.",
            STORE_CAP_MIN >> 20,
            STORE_CAP_MAX >> 20
        ));
    }
    Ok(bytes)
}

#[derive(Serialize)]
pub struct HistoryRow {
    pub t: u64,
    pub kind: String,
    pub app: String,
    pub receipt: Option<String>,
    pub status: Option<String>,
    pub fixed_in: Option<String>,
    pub path: Option<String>,
}

/// Apps that can be excluded: everything installed plus anything already
/// excluded or holding stored data, so an exclusion can always be undone.
fn app_choices(excluded: &[String]) -> Vec<AppChoice> {
    let installed: Vec<String> = installed_apps()
        .map(|(_, apps)| apps.into_iter().map(|a| a.id).collect())
        .unwrap_or_default();
    let stored = Reader::open(store_root()).apps();
    let mut ids: Vec<String> = installed
        .iter()
        .chain(stored.iter())
        .chain(excluded.iter())
        .filter(|a| valid_app_id(a))
        .cloned()
        .collect();
    ids.sort();
    ids.dedup();
    ids.into_iter()
        .map(|id| AppChoice {
            installed: installed.contains(&id),
            excluded: excluded.contains(&id),
            id,
        })
        .collect()
}

#[tauri::command]
pub fn get_breadbug_settings() -> BreadbugSettings {
    let doc = load_doc(&breadbug_toml());
    let excluded = get_str_list(&doc, &["excluded_apps"]);
    let queued = Queue::open(&state_dir())
        .map(|q| q.list().len())
        .unwrap_or(0);
    BreadbugSettings {
        breadbug_installed: super::util::command_exists("breadbug"),
        upload_enabled: get_bool(&doc, &["upload_enabled"]).unwrap_or(false),
        notify_fixed: get_bool(&doc, &["notify_fixed"]).unwrap_or(false),
        endpoint: get_str(&doc, &["endpoint"]).unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
        apps: app_choices(&excluded),
        store_bytes: Reader::open(store_root())
            .usage()
            .map(|u| u.total_bytes)
            .unwrap_or(0),
        // breadd applies the same default when the key is absent.
        store_cap_bytes: stored_cap(&doc).unwrap_or_else(|| StoreLimits::default().max_total_bytes),
        store_cap_default_bytes: StoreLimits::default().max_total_bytes,
        queued,
        consent_text: CONSENT_TEXT.to_string(),
    }
}

/// The cap recorded in breadbug.toml, when it is a usable positive number.
fn stored_cap(doc: &toml_edit::DocumentMut) -> Option<u64> {
    get_i64(doc, &["store", "max_total_bytes"])
        .filter(|v| *v > 0)
        .map(|v| v as u64)
}

/// Validate and normalise an exclusion list: valid app ids only, sorted,
/// without duplicates.
pub fn clean_exclusions(apps: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for a in apps {
        if !valid_app_id(a) {
            return Err(format!("'{a}' is not a valid app id"));
        }
        out.push(a.clone());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Edit `breadbug.toml` in place: only the keys the page models change. The
/// other `[store]` bounds keep their values, and their comments.
pub fn apply_settings(
    doc: &mut toml_edit::DocumentMut,
    upload_enabled: bool,
    notify_fixed: bool,
    excluded: &[String],
    store_cap_bytes: u64,
) {
    set_bool(doc, &["upload_enabled"], upload_enabled);
    set_bool(doc, &["notify_fixed"], notify_fixed);
    set_str_list(doc, &["excluded_apps"], excluded);
    set_i64(doc, &["store", "max_total_bytes"], store_cap_bytes as i64);
}

/// Ask breadd to apply the exclusions now. Best effort: without a running
/// daemon the file is simply read at its next start.
fn tell_daemon() {
    let socket = bread_bug::ipc::default_socket_path();
    let _ = bread_bug::ipc::request(
        &socket,
        "bug.reload",
        serde_json::json!({}),
        std::time::Duration::from_secs(2),
    );
}

#[tauri::command]
pub fn save_breadbug_settings(
    upload_enabled: bool,
    notify_fixed: bool,
    excluded_apps: Vec<String>,
    store_cap_bytes: u64,
) -> Result<(), String> {
    let excluded = clean_exclusions(&excluded_apps)?;
    let cap = clean_store_cap(store_cap_bytes)?;
    let path = breadbug_toml();
    let mut doc = load_doc(&path);
    apply_settings(&mut doc, upload_enabled, notify_fixed, &excluded, cap);
    save_doc(&path, &doc).map_err(|e| e.to_string())?;
    tell_daemon();
    Ok(())
}

#[tauri::command]
pub fn breadbug_history() -> Vec<HistoryRow> {
    let Ok(h) = History::open(&state_dir()) else {
        return Vec::new();
    };
    let mut rows: Vec<HistoryRow> = h
        .list()
        .into_iter()
        .map(|e| HistoryRow {
            t: e.t,
            kind: match e.kind {
                HistoryKind::Sent => "sent".into(),
                HistoryKind::Saved => "saved".into(),
            },
            app: e.app,
            // the deletion token never leaves the backend
            receipt: e.receipt,
            status: e.status,
            fixed_in: e.fixed_in,
            path: e.path,
        })
        .collect();
    rows.reverse();
    rows
}

/// Delete a report from the server using the token kept in history. Needs
/// sending to be on, like every network action in breadbug.
#[tauri::command]
pub fn breadbug_delete_report(receipt: String) -> Result<String, String> {
    let doc = load_doc(&breadbug_toml());
    if !get_bool(&doc, &["upload_enabled"]).unwrap_or(false) {
        return Err(
            "Sending is turned off, so breadbug will not contact the server. Turn it on first."
                .into(),
        );
    }
    let h = History::open(&state_dir()).map_err(|e| e.to_string())?;
    let entry = h
        .find(&receipt)
        .ok_or("That report is not in your history.")?;
    let token = entry
        .deletion_token
        .ok_or("This report has no deletion token on this computer.")?;
    let endpoint = get_str(&doc, &["endpoint"]).unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());
    let client = Client::new(&endpoint).map_err(|e| e.to_string())?;
    match client.delete(&receipt, &token) {
        Ok(found) => {
            h.remove(&receipt).map_err(|e| e.to_string())?;
            Ok(if found {
                "Deleted from the server.".into()
            } else {
                "The server no longer had it. Removed from your history.".into()
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Drop everything waiting in the offline queue. Returns how many.
#[tauri::command]
pub fn breadbug_clear_queue() -> usize {
    Queue::open(&state_dir()).map(|q| q.clear()).unwrap_or(0)
}

/// Delete everything breadbug has stored locally: logs, events and markers.
/// Uses the daemon when it is up (so its in-memory state agrees), otherwise
/// removes the files directly.
#[tauri::command]
pub fn breadbug_wipe_local() -> Result<(), String> {
    let socket = bread_bug::ipc::default_socket_path();
    let via_daemon = bread_bug::ipc::request(
        &socket,
        "bug.wipe",
        serde_json::json!({}),
        std::time::Duration::from_secs(3),
    );
    if via_daemon.is_ok() {
        return Ok(());
    }
    let apps = store_root().join("apps");
    match std::fs::remove_dir_all(&apps) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Could not delete {}: {e}", apps.display())),
    }
}

/// Open the reporter in a terminal (the same command the Super+Shift+B bind runs).
#[tauri::command]
pub fn open_breadbug() -> Result<(), String> {
    if !super::util::command_exists("breadbug") {
        return Err("breadbug is not installed.".into());
    }
    std::process::Command::new("kitty")
        .args([
            "--class",
            "breadbug",
            "--title",
            "Report a problem",
            "breadbug",
        ])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open a terminal: {e}"))
}

/// Open the plain-language statement of what a report contains, what is
/// replaced and what is never collected. It lives on the BOS site; there is no
/// in-app copy to drift from it.
pub const PRIVACY_URL: &str = "https://bos.breadway.dev/#reports";

#[tauri::command]
pub fn open_breadbug_privacy(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(PRIVACY_URL, None::<&str>)
        .map_err(|e| format!("Could not open the browser: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_are_validated_sorted_and_deduplicated() {
        let got = clean_exclusions(&["search".into(), "clip".into(), "clip".into()]).unwrap();
        assert_eq!(got, ["clip", "search"]);
        for bad in ["../x", "Clip", "a b", "", "x;rm"] {
            assert!(clean_exclusions(&[bad.to_string()]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn saving_changes_only_the_modelled_keys_and_keeps_everything_else() {
        let mut doc: toml_edit::DocumentMut =
            "# my notes\nendpoint = \"https://example.test\"\n\n[store]\nmax_apps = 5\n"
                .parse()
                .unwrap();
        apply_settings(&mut doc, true, false, &["clip".into()], 8 << 20);
        let text = doc.to_string();
        assert!(text.contains("# my notes"));
        assert!(text.contains("endpoint = \"https://example.test\""));
        assert!(text.contains("max_apps = 5"));
        assert!(text.contains("max_total_bytes = 8388608"));
        assert!(text.contains("upload_enabled = true"));
        assert!(text.contains("notify_fixed = false"));
        assert!(text.contains("excluded_apps = [\"clip\"]"));
    }

    #[test]
    fn store_cap_is_bounded() {
        assert_eq!(clean_store_cap(8 << 20).unwrap(), 8 << 20);
        assert_eq!(clean_store_cap(STORE_CAP_MIN).unwrap(), STORE_CAP_MIN);
        assert_eq!(clean_store_cap(STORE_CAP_MAX).unwrap(), STORE_CAP_MAX);
        for bad in [0, STORE_CAP_MIN - 1, STORE_CAP_MAX + 1, u64::MAX] {
            assert!(clean_store_cap(bad).is_err(), "{bad} was accepted");
        }
    }

    #[test]
    fn stored_cap_reads_a_positive_value_only() {
        let doc: toml_edit::DocumentMut = "[store]\nmax_total_bytes = 4194304\n".parse().unwrap();
        assert_eq!(stored_cap(&doc), Some(4 << 20));
        for text in [
            "[store]\nmax_total_bytes = 0\n",
            "[store]\nmax_total_bytes = -1\n",
            "",
        ] {
            let doc: toml_edit::DocumentMut = text.parse().unwrap();
            assert_eq!(stored_cap(&doc), None, "{text:?}");
        }
    }

    #[test]
    fn defaults_when_nothing_is_set() {
        let doc: toml_edit::DocumentMut = "".parse().unwrap();
        assert!(!get_bool(&doc, &["upload_enabled"]).unwrap_or(false));
        assert!(get_str_list(&doc, &["excluded_apps"]).is_empty());
    }
}
