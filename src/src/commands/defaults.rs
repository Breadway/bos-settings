//! Default applications via `~/.config/mimeapps.list`. Categories cover
//! the associations BOS already ships in skel (browser, files, images,
//! PDF, editor) plus a terminal entry.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use serde::Serialize;

use super::config;

const TERMINAL_MIMES: &[&str] = &["x-scheme-handler/terminal"];

const CATEGORIES: &[(&str, &[&str])] = &[
    (
        "browser",
        &[
            "x-scheme-handler/http",
            "x-scheme-handler/https",
            "text/html",
        ],
    ),
    ("files", &["inode/directory"]),
    ("terminal", TERMINAL_MIMES),
    (
        "image",
        &[
            "image/png",
            "image/jpeg",
            "image/webp",
            "image/gif",
            "image/svg+xml",
        ],
    ),
    ("pdf", &["application/pdf"]),
    ("editor", &["text/plain", "text/markdown"]),
];

#[derive(Serialize, Clone)]
pub struct DesktopApp {
    id: String,
    name: String,
}

#[derive(Serialize)]
pub struct DefaultsStatus {
    path: String,
    current: HashMap<String, String>,
    options: HashMap<String, Vec<DesktopApp>>,
}

fn mimeapps_path() -> PathBuf {
    config::config_dir().join("mimeapps.list")
}

fn xdg_terminals_path() -> PathBuf {
    config::config_dir().join("xdg-terminals.list")
}

fn applications_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }
    dirs
}

#[derive(Clone)]
struct DesktopMeta {
    id: String,
    name: String,
    exec: Option<String>,
    mimes: Vec<String>,
    terminal: bool,
}

/// First word of an Exec line with any path and quotes removed, e.g.
/// `/usr/bin/kitty --single-instance` -> `kitty`.
fn command_binary(command: &str) -> Option<String> {
    let first = command.split_whitespace().next()?;
    let first = first.trim_matches(|c| c == '"' || c == '\'');
    let name = first.rsplit('/').next()?;
    (!name.is_empty()).then(|| name.to_string())
}

fn parse_desktop(id: &str, text: &str) -> Option<DesktopMeta> {
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = None;
    let mut mimes = Vec::new();
    let mut terminal = false;
    let mut hidden = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(v) = line.strip_prefix("Name=") {
            if name.is_empty() {
                name = v.to_string();
            }
        } else if let Some(v) = line.strip_prefix("Exec=") {
            if exec.is_none() {
                exec = command_binary(v);
            }
        } else if let Some(v) = line.strip_prefix("MimeType=") {
            mimes = v
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        } else if let Some(v) = line.strip_prefix("Categories=") {
            terminal |= v.split(';').any(|c| c.trim() == "TerminalEmulator");
        } else if line == "Hidden=true" || line == "NoDisplay=true" {
            hidden = true;
        }
    }
    if hidden || name.is_empty() {
        return None;
    }
    Some(DesktopMeta {
        id: id.to_string(),
        name,
        exec,
        mimes,
        terminal,
    })
}

fn scan_desktops() -> Vec<DesktopMeta> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in applications_dirs() {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Some(id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !seen.insert(id.to_string()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(meta) = parse_desktop(id, &text) {
                out.push(meta);
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn parse_default_applications(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut in_defaults = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_defaults = t.eq_ignore_ascii_case("[Default Applications]");
            continue;
        }
        if !in_defaults || t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let desktop = v.split(';').next().unwrap_or("").trim();
            if !desktop.is_empty() {
                map.insert(k.trim().to_string(), desktop.to_string());
            }
        }
    }
    map
}

fn current_for_category(defaults: &BTreeMap<String, String>, mimes: &[&str]) -> String {
    for mime in mimes {
        if let Some(v) = defaults.get(*mime) {
            return v.clone();
        }
    }
    String::new()
}

fn options_for(
    apps: &[DesktopMeta],
    category: &str,
    mimes: &[&str],
    current: &str,
) -> Vec<DesktopApp> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for app in apps {
        let matches = if category == "terminal" {
            app.terminal || app.mimes.iter().any(|m| mimes.contains(&m.as_str()))
        } else {
            app.mimes.iter().any(|m| mimes.contains(&m.as_str()))
        };
        if matches && seen.insert(app.id.clone()) {
            out.push(DesktopApp {
                id: app.id.clone(),
                name: app.name.clone(),
            });
        }
    }
    if !current.is_empty() && !seen.contains(current) {
        out.insert(
            0,
            DesktopApp {
                id: current.to_string(),
                name: hidden_desktop_name(current)
                    .unwrap_or_else(|| current.trim_end_matches(".desktop").to_string()),
            },
        );
    }
    out
}

/// Name of a desktop entry the scan skips (NoDisplay/Hidden). xdg-mime and
/// browsers register defaults as hidden `userapp-*.desktop` files, which
/// otherwise showed as raw ids like "userapp-Zen-W24BP3".
fn hidden_desktop_name(id: &str) -> Option<String> {
    if !valid_desktop_id(id) {
        return None;
    }
    applications_dirs().into_iter().find_map(|dir| {
        let text = std::fs::read_to_string(dir.join(id)).ok()?;
        desktop_entry_name(&text)
    })
}

fn desktop_entry_name(text: &str) -> Option<String> {
    let mut in_entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
        } else if in_entry {
            if let Some(v) = line.strip_prefix("Name=") {
                return Some(v.to_string()).filter(|v| !v.is_empty());
            }
        }
    }
    None
}

#[tauri::command]
pub fn get_default_apps() -> DefaultsStatus {
    let path = mimeapps_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let defaults = parse_default_applications(&text);
    let apps = scan_desktops();
    let terminals_text = std::fs::read_to_string(xdg_terminals_path()).ok();
    let bound_terminal = super::keybinds::terminal_bind_command();
    let mut current = HashMap::new();
    let mut options = HashMap::new();
    for (cat, mimes) in CATEGORIES {
        let cur = if *cat == "terminal" {
            effective_terminal(
                &defaults,
                terminals_text.as_deref(),
                &apps,
                bound_terminal.as_deref(),
            )
        } else {
            current_for_category(&defaults, mimes)
        };
        options.insert((*cat).to_string(), options_for(&apps, cat, mimes, &cur));
        current.insert((*cat).to_string(), cur);
    }
    DefaultsStatus {
        path: path.display().to_string(),
        current,
        options,
    }
}

fn is_terminal(app: &DesktopMeta) -> bool {
    app.terminal || app.mimes.iter().any(|m| m == TERMINAL_MIMES[0])
}

fn matches_terminal_binary(app: &DesktopMeta, binary: &str) -> bool {
    let stem = app.id.trim_end_matches(".desktop");
    app.exec.as_deref() == Some(binary) || stem.eq_ignore_ascii_case(binary)
}

/// An explicit user choice: xdg-terminals.list first, then the terminal MIME
/// association. Explicit choices are returned as written, even if the
/// desktop entry has since been removed, because that is what the file says.
fn configured_terminal(
    defaults: &BTreeMap<String, String>,
    terminals_text: Option<&str>,
) -> String {
    let listed = terminals_text
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .unwrap_or("");
    if !listed.is_empty() {
        return listed.to_string();
    }
    current_for_category(defaults, TERMINAL_MIMES)
}

/// The terminal the desktop will actually use when nothing is configured.
/// Only a terminal whose desktop entry is installed can be inferred, and an
/// ambiguous machine (several terminals, no explicit choice or Super+Enter
/// bind) stays unset rather than guessing.
fn effective_terminal(
    defaults: &BTreeMap<String, String>,
    terminals_text: Option<&str>,
    apps: &[DesktopMeta],
    bound_command: Option<&str>,
) -> String {
    let configured = configured_terminal(defaults, terminals_text);
    if !configured.is_empty() {
        return configured;
    }
    if let Some(binary) = bound_command.and_then(command_binary) {
        if let Some(app) = apps
            .iter()
            .find(|a| is_terminal(a) && matches_terminal_binary(a, &binary))
        {
            return app.id.clone();
        }
    }
    let terminals: Vec<&DesktopMeta> = apps.iter().filter(|a| is_terminal(a)).collect();
    if terminals.len() == 1 {
        return terminals[0].id.clone();
    }
    String::new()
}

#[derive(serde::Deserialize)]
pub struct SaveDefaultsInput {
    current: HashMap<String, String>,
}

#[tauri::command]
pub fn save_default_apps(input: SaveDefaultsInput) -> Result<(), String> {
    let path = mimeapps_path();
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let mut replacements = BTreeMap::new();
    for (cat, mimes) in CATEGORIES {
        let Some(desktop) = input.current.get(*cat).map(|s| s.trim()) else {
            continue;
        };
        if desktop.is_empty() {
            continue;
        }
        if !valid_desktop_id(desktop) {
            return Err(format!("invalid desktop id '{desktop}'"));
        }
        for mime in *mimes {
            replacements.insert((*mime).to_string(), desktop.to_string());
        }
        if *cat == "terminal" {
            write_terminal_list(desktop)?;
        }
    }
    let text = upsert_defaults(&existing, &replacements);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    config::atomic_write(&path, &text).map_err(|e| e.to_string())
}

fn write_terminal_list(desktop: &str) -> Result<(), String> {
    let path = xdg_terminals_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    config::atomic_write(&path, &format!("{desktop}\n")).map_err(|e| e.to_string())
}

fn valid_desktop_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.ends_with(b".desktop")
        && bytes.len() > ".desktop".len()
        && bytes.len() <= 128
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_' | b'.' | b'+'))
}

fn upsert_defaults(existing: &str, replacements: &BTreeMap<String, String>) -> String {
    if existing.trim().is_empty() {
        let mut out = String::from("[Default Applications]\n");
        for (mime, desktop) in replacements {
            out.push_str(&format!("{mime}={desktop}\n"));
        }
        return out;
    }
    let mut out = String::new();
    let mut in_defaults = false;
    let mut seen = std::collections::HashSet::new();
    let mut wrote_header = false;
    for line in existing.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            if in_defaults {
                for (mime, desktop) in replacements {
                    if seen.insert(mime.clone()) {
                        out.push_str(&format!("{mime}={desktop}\n"));
                    }
                }
            }
            in_defaults = t.eq_ignore_ascii_case("[Default Applications]");
            if in_defaults {
                wrote_header = true;
            }
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_defaults {
            if let Some((k, _)) = t.split_once('=') {
                let key = k.trim();
                if let Some(desktop) = replacements.get(key) {
                    out.push_str(&format!("{key}={desktop}\n"));
                    seen.insert(key.to_string());
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if in_defaults {
        for (mime, desktop) in replacements {
            if seen.insert(mime.clone()) {
                out.push_str(&format!("{mime}={desktop}\n"));
            }
        }
    } else if !wrote_header {
        if !out.ends_with('\n') && !out.is_empty() {
            out.push('\n');
        }
        out.push_str("\n[Default Applications]\n");
        for (mime, desktop) in replacements {
            out.push_str(&format!("{mime}={desktop}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skel_defaults() {
        let text = "\
[Default Applications]
text/html=zen.desktop
x-scheme-handler/http=zen.desktop
inode/directory=org.gnome.Nautilus.desktop
";
        let map = parse_default_applications(text);
        assert_eq!(map.get("text/html").unwrap(), "zen.desktop");
        assert_eq!(
            current_for_category(&map, &["x-scheme-handler/http", "text/html"]),
            "zen.desktop"
        );
    }

    #[test]
    fn upsert_replaces_only_named_keys() {
        let existing = "\
# keep
[Default Applications]
text/html=old.desktop
image/png=org.gnome.Loupe.desktop

[Added Associations]
text/html=extra.desktop;
";
        let mut rep = BTreeMap::new();
        rep.insert("text/html".into(), "zen.desktop".into());
        rep.insert("x-scheme-handler/http".into(), "zen.desktop".into());
        let out = upsert_defaults(existing, &rep);
        assert!(out.contains("# keep"));
        assert!(out.contains("text/html=zen.desktop"));
        assert!(out.contains("x-scheme-handler/http=zen.desktop"));
        assert!(out.contains("image/png=org.gnome.Loupe.desktop"));
        assert!(out.contains("[Added Associations]"));
        assert!(out.contains("text/html=extra.desktop;"));
        assert_eq!(out.matches("text/html=zen.desktop").count(), 1);
    }

    #[test]
    fn hidden_entry_names_are_read() {
        assert_eq!(
            desktop_entry_name("[Desktop Entry]\nNoDisplay=true\nName=Zen\n[Desktop Action new]\nName=New window\n"),
            Some("Zen".into())
        );
        assert_eq!(desktop_entry_name("[Desktop Action x]\nName=Nope\n"), None);
    }

    #[test]
    fn desktop_id_check() {
        assert!(valid_desktop_id("zen.desktop"));
        assert!(valid_desktop_id("org.gnome.Nautilus.desktop"));
        assert!(!valid_desktop_id("zen"));
        assert!(!valid_desktop_id("../evil.desktop"));
    }

    fn terminal_app(id: &str, exec: &str) -> DesktopMeta {
        DesktopMeta {
            id: id.into(),
            name: id.trim_end_matches(".desktop").into(),
            exec: command_binary(exec),
            mimes: vec![],
            terminal: true,
        }
    }

    #[test]
    fn parse_desktop_records_the_exec_binary() {
        let app = parse_desktop(
            "kitty.desktop",
            "[Desktop Entry]\nName=kitty\nExec=/usr/bin/kitty --single-instance\nCategories=TerminalEmulator;\n",
        )
        .unwrap();
        assert_eq!(app.exec.as_deref(), Some("kitty"));
    }

    #[test]
    fn only_installed_terminal_is_detected_when_nothing_is_configured() {
        let apps = vec![terminal_app("kitty.desktop", "kitty")];
        assert_eq!(
            effective_terminal(&BTreeMap::new(), None, &apps, None),
            "kitty.desktop"
        );
    }

    #[test]
    fn missing_terminal_is_not_claimed() {
        let apps: Vec<DesktopMeta> = vec![];
        assert_eq!(effective_terminal(&BTreeMap::new(), None, &apps, None), "");
    }

    #[test]
    fn bound_terminal_is_used_when_it_is_installed() {
        let apps = vec![
            terminal_app("kitty.desktop", "kitty"),
            terminal_app("foot.desktop", "foot"),
        ];
        assert_eq!(
            effective_terminal(&BTreeMap::new(), None, &apps, Some("foot --server")),
            "foot.desktop"
        );
    }

    #[test]
    fn a_bound_terminal_that_is_not_installed_is_not_claimed() {
        let apps = vec![terminal_app("foot.desktop", "foot")];
        assert_eq!(
            effective_terminal(&BTreeMap::new(), None, &apps, Some("kitty")),
            "foot.desktop",
            "fall back to the one installed terminal, never to kitty"
        );
    }

    #[test]
    fn explicit_user_default_wins_over_detection() {
        let apps = vec![
            terminal_app("kitty.desktop", "kitty"),
            terminal_app("foot.desktop", "foot"),
        ];
        assert_eq!(
            effective_terminal(
                &BTreeMap::new(),
                Some("foot.desktop\n"),
                &apps,
                Some("kitty")
            ),
            "foot.desktop"
        );
    }

    #[test]
    fn mimeapps_terminal_default_is_also_explicit() {
        let mut defaults = BTreeMap::new();
        defaults.insert("x-scheme-handler/terminal".into(), "foot.desktop".into());
        let apps = vec![
            terminal_app("kitty.desktop", "kitty"),
            terminal_app("foot.desktop", "foot"),
        ];
        assert_eq!(
            effective_terminal(&defaults, None, &apps, None),
            "foot.desktop"
        );
    }

    #[test]
    fn several_installed_terminals_without_a_choice_stay_unset() {
        let apps = vec![
            terminal_app("kitty.desktop", "kitty"),
            terminal_app("foot.desktop", "foot"),
        ];
        assert_eq!(effective_terminal(&BTreeMap::new(), None, &apps, None), "");
    }

    #[test]
    fn command_binary_handles_paths_and_quotes() {
        assert_eq!(command_binary("kitty --foo").as_deref(), Some("kitty"));
        assert_eq!(command_binary("\"/usr/bin/foot\"").as_deref(), Some("foot"));
        assert_eq!(command_binary("   "), None);
    }

    #[test]
    fn parse_desktop_skips_hidden() {
        let hidden = parse_desktop(
            "x.desktop",
            "[Desktop Entry]\nName=X\nNoDisplay=true\nMimeType=text/plain;\n",
        );
        assert!(hidden.is_none());
        let ok = parse_desktop(
            "ed.desktop",
            "[Desktop Entry]\nName=Editor\nMimeType=text/plain;\nCategories=Utility;\n",
        )
        .unwrap();
        assert_eq!(ok.name, "Editor");
        assert!(ok.mimes.contains(&"text/plain".into()));
    }
}

#[tauri::command]
pub fn list_startup_apps() -> Vec<DesktopApp> {
    scan_desktops()
        .into_iter()
        .map(|a| DesktopApp {
            id: a.id,
            name: a.name,
        })
        .collect()
}
