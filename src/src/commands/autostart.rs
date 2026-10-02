//! hypr/autostart.json: the *extra*, user-toggleable autostart apps. The
//! core bootstrap sequence stays hardcoded in hyprland.lua on purpose (it's
//! timing/order-sensitive infrastructure, not something this panel exposes).
//!
//! Other panels that need something started at login (night light) add a
//! tagged entry here through [`upsert_entry`] / [`remove_entries`] rather
//! than writing hyprlang fragments: BOS loads `hyprland.lua`, which never
//! reads `hyprland.conf`.

use serde::{Deserialize, Serialize};

use super::config;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AutostartEntry {
    command: String,
    #[serde(default)]
    label: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

impl AutostartEntry {
    pub fn new(command: impl Into<String>, label: impl Into<String>, enabled: bool) -> Self {
        Self {
            command: command.into(),
            label: label.into(),
            enabled,
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct AutostartFile {
    #[serde(default)]
    extra: Vec<AutostartEntry>,
}

/// Must stay in sync with `DEFAULT_EXTRA` in BOS's
/// `scripts/system/autostart.lua`: that is what the session runs when
/// autostart.json is missing, so a first save from here must start from the
/// same list or it would silently drop the command buses.
fn default_extra() -> Vec<AutostartEntry> {
    let bus = |cmd: &str, label: &str| {
        AutostartEntry::new(
            format!("bash -c 'command -v {cmd} >/dev/null && exec {cmd} listen'"),
            label,
            true,
        )
    };
    vec![
        AutostartEntry::new("breadbar", "Bar (breadbar)", true),
        AutostartEntry::new("hypridle", "Idle / lock daemon (hypridle)", true),
        AutostartEntry::new("bos-netcheck", "Network connectivity check", true),
        AutostartEntry::new(
            "bash -c 'command -v bos-first-boot >/dev/null && exec bos-first-boot'",
            "First-boot hardware probe",
            true,
        ),
        AutostartEntry::new(
            "breadhelp --autostart",
            "BOS Help (first-run onboarding)",
            true,
        ),
        bus("breadpaper", "Wallpaper command bus (breadpaper listen)"),
        bus("breadshot", "Screenshot command bus (breadshot listen)"),
        bus("breadlock", "Lock command bus (breadlock listen)"),
        bus("breadbox", "Launcher command bus (breadbox listen)"),
        bus("breadhelp", "Help command bus (breadhelp listen)"),
        bus("breadpad", "Capture command bus (breadpad listen)"),
    ]
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/autostart.json")
}

/// The entries as the session will see them (the stock list when the file is
/// missing or unreadable).
pub fn load_entries() -> Vec<AutostartEntry> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str::<AutostartFile>(&s).ok())
        .map(|f| f.extra)
        .unwrap_or_else(default_extra)
}

pub fn store_entries(entries: Vec<AutostartEntry>) -> Result<(), String> {
    let file = AutostartFile { extra: entries };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}

/// Replace the first entry matching `is_mine` with `entry` (and drop any
/// further matches), or append `entry` when none matches. Pure, for tests.
pub fn upsert_entry(
    entries: &mut Vec<AutostartEntry>,
    is_mine: impl Fn(&AutostartEntry) -> bool,
    entry: AutostartEntry,
) {
    let mut placed = false;
    let mut out = Vec::with_capacity(entries.len() + 1);
    for e in entries.drain(..) {
        if is_mine(&e) {
            if !placed {
                out.push(entry.clone());
                placed = true;
            }
        } else {
            out.push(e);
        }
    }
    if !placed {
        out.push(entry);
    }
    *entries = out;
}

/// Drop every entry matching `is_mine`; returns whether anything was removed.
pub fn remove_entries(
    entries: &mut Vec<AutostartEntry>,
    is_mine: impl Fn(&AutostartEntry) -> bool,
) -> bool {
    let before = entries.len();
    entries.retain(|e| !is_mine(e));
    entries.len() != before
}

#[tauri::command]
pub fn get_autostart_entries() -> Vec<AutostartEntry> {
    load_entries()
}

/// Empty-command rows (still-being-typed "Add app" entries) are dropped on
/// save rather than written as a broken autostart.json entry the Lua loader
/// would otherwise have to reject.
#[tauri::command]
pub fn save_autostart_entries(
    entries: Vec<AutostartEntry>,
    allow_system_changes: Option<bool>,
) -> Result<(), String> {
    if !allow_system_changes.unwrap_or(false) {
        check_system_entries(&load_entries(), &entries)?;
    }
    let entries: Vec<AutostartEntry> = entries
        .into_iter()
        .filter(|e| !e.command.trim().is_empty())
        .collect();
    store_entries(entries)
}

fn is_system_entry(entry: &AutostartEntry) -> bool {
    let command = entry.command();
    default_extra().iter().any(|e| e.command() == command)
        || command == "hyprsunset"
        || command
            .strip_prefix("hyprsunset -t ")
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        || ["breadclip", "breadsearch", "bread"].iter().any(|cmd| {
            command == format!("bash -c 'command -v {cmd} >/dev/null && exec {cmd} listen'")
        })
}

fn check_system_entries(before: &[AutostartEntry], after: &[AutostartEntry]) -> Result<(), String> {
    let mut remaining = after.to_vec();
    for entry in before.iter().filter(|e| is_system_entry(e)) {
        let Some(index) = remaining.iter().position(|e| e == entry) else {
            return Err("Changing core startup services requires advanced confirmation.".into());
        };
        remaining.remove(index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_sunset(e: &AutostartEntry) -> bool {
        e.command().split_whitespace().next() == Some("hyprsunset")
    }

    #[test]
    fn core_entries_need_confirmation_to_change() {
        let before = default_extra();
        assert!(check_system_entries(&before, &before).is_ok());
        let mut removed = before.clone();
        removed.remove(0);
        assert!(check_system_entries(&before, &removed).is_err());
        let mut disabled = before.clone();
        disabled[0].enabled = false;
        assert!(check_system_entries(&before, &disabled).is_err());
        let mut added = before.clone();
        added.push(AutostartEntry::new("kitty", "Terminal", true));
        assert!(check_system_entries(&before, &added).is_ok());
    }

    #[test]
    fn user_commands_are_not_core_by_prefix() {
        assert!(!is_system_entry(&AutostartEntry::new(
            "breadbar --custom",
            "My bar",
            true
        )));
        assert!(is_system_entry(&AutostartEntry::new(
            "hyprsunset -t 3500",
            "Night light",
            true
        )));
    }

    #[test]
    fn default_list_matches_the_loader_stock_list() {
        let d = default_extra();
        assert_eq!(d.len(), 11);
        assert_eq!(d[0].command(), "breadbar");
        assert!(d
            .iter()
            .any(|e| e.command()
                == "bash -c 'command -v breadpad >/dev/null && exec breadpad listen'"));
        assert!(d.iter().all(|e| e.enabled));
    }

    #[test]
    fn upsert_appends_then_replaces_in_place() {
        let mut v = vec![AutostartEntry::new("breadbar", "Bar", true)];
        upsert_entry(
            &mut v,
            is_sunset,
            AutostartEntry::new("hyprsunset -t 3500", "Night light", true),
        );
        assert_eq!(v.len(), 2);
        upsert_entry(
            &mut v,
            is_sunset,
            AutostartEntry::new("hyprsunset -t 2800", "Night light", true),
        );
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].command(), "hyprsunset -t 2800");
        assert_eq!(v[0].command(), "breadbar");
    }

    #[test]
    fn upsert_collapses_duplicates() {
        let mut v = vec![
            AutostartEntry::new("hyprsunset", "a", true),
            AutostartEntry::new("breadbar", "Bar", true),
            AutostartEntry::new("hyprsunset -t 3000", "b", true),
        ];
        upsert_entry(
            &mut v,
            is_sunset,
            AutostartEntry::new("hyprsunset -t 3500", "n", true),
        );
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].command(), "hyprsunset -t 3500");
    }

    #[test]
    fn remove_reports_whether_it_changed_anything() {
        let mut v = vec![
            AutostartEntry::new("breadbar", "Bar", true),
            AutostartEntry::new("hyprsunset -t 3500", "n", true),
        ];
        assert!(remove_entries(&mut v, is_sunset));
        assert!(!remove_entries(&mut v, is_sunset));
        assert_eq!(v.len(), 1);
    }
}
