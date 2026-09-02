//! `~/.config/bread/shell.toml` — the `active = "<id>"` key selects the
//! **shell theme**: breadbar/breadbox chrome, layout, and animation style
//! (`liquid-motion`, `glass-workbench`, `spotlight`, `daylight`). This is a
//! single global selector; per-monitor differences come from the wallpaper
//! and its pywal palette, not from the shell theme.
//!
//! `bread_theme::shell::list()` enumerates every discoverable theme (user
//! config → system dir → compiled-in builtins). Setting `active` is a
//! non-destructive `toml_edit` write.
//!
//! A running breadbar watches `shell.toml` (`bread_theme::shell::watch()`)
//! and, on an `active` change, re-execs itself so the full theme applies —
//! geometry and widget structure, not just CSS tokens. breadbox re-reads
//! the theme on each launcher open. So a plain write here is normally
//! enough; `restart_shell_apps` stays as a manual fallback (an old breadbar
//! without self-restart, or a failed re-exec).

use serde::Serialize;

use super::config;

fn shell_toml_path() -> std::path::PathBuf {
    config::config_dir().join("bread/shell.toml")
}

/// Default when `shell.toml` is missing or has no `active` — matches
/// `bread_theme::shell`'s own hardcoded fallback.
const DEFAULT_THEME_ID: &str = "liquid-motion";

#[derive(Serialize)]
pub struct ShellThemeInfo {
    id: String,
    name: String,
    /// `"builtin"`, `"system"`, or `"user"` — for a source badge in the picker.
    source: String,
}

#[tauri::command]
pub fn list_shell_themes() -> Vec<ShellThemeInfo> {
    bread_theme::shell::list()
        .into_iter()
        .map(|t| ShellThemeInfo {
            id: t.id,
            name: t.name,
            source: match t.source {
                bread_theme::shell::ThemeSource::User => "user",
                bread_theme::shell::ThemeSource::System => "system",
                bread_theme::shell::ThemeSource::Builtin => "builtin",
            }
            .to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn get_active_shell_theme() -> String {
    let doc = config::load_doc(&shell_toml_path());
    doc.get("active")
        .and_then(|i| i.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| DEFAULT_THEME_ID.to_string())
}

/// Writes `active = "<id>"` into `shell.toml`, preserving any other keys and
/// comments. Rejects an id that isn't a discoverable theme — a typo would
/// silently fall breadbar/breadbox back to the builtin with no error.
#[tauri::command]
pub fn set_active_shell_theme(id: String) -> Result<(), String> {
    if !bread_theme::shell::list().iter().any(|t| t.id == id) {
        return Err(format!("no shell theme with id '{id}'"));
    }
    let path = shell_toml_path();
    let mut doc = config::load_doc(&path);
    doc["active"] = toml_edit::value(id);
    config::save_doc(&path, &doc).map_err(|e| e.to_string())
}

/// Restart breadbar and breadbox so a shell-theme change takes full effect:
/// breadbar's live watch handles colours/tokens but not window geometry
/// (anchors, exclusive zone), and breadbox has no watch at all. Detached so
/// the new processes outlive this settings process. Best-effort — a missing
/// binary or a compositor without them running is not an error.
#[tauri::command]
pub fn restart_shell_apps() -> Result<(), String> {
    for app in ["breadbar", "breadbox"] {
        let _ = std::process::Command::new("pkill").args(["-x", app]).status();
    }
    std::thread::sleep(std::time::Duration::from_millis(250));
    spawn_detached("breadbar", &[]);
    // Matches `~/.config/hypr/autostart.json` — breadbox runs as a command
    // bus (`breadbox listen`); the launcher window is spawned on demand.
    spawn_detached("breadbox", &["listen"]);
    Ok(())
}

fn spawn_detached(prog: &str, args: &[&str]) {
    use std::process::{Command, Stdio};
    // `setsid -f` forks the target into its own session and exits at once;
    // the target is reparented to init, so it never becomes a zombie of
    // this settings process. `.status()` reaps `setsid` itself (it returns
    // immediately) so that doesn't linger either — a plain `.spawn()` here
    // leaves a `<defunct>` entry behind for every restart.
    let _ = Command::new("setsid")
        .arg("-f")
        .arg(prog)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// `set`-style write must not drop sibling keys or comments in shell.toml.
    #[test]
    fn writing_active_preserves_other_content() {
        let dir = std::env::temp_dir().join(format!("bos-shelltoml-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shell.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            "# my shell config\nactive = \"spotlight\"\n\n[experimental]\nfoo = true\n"
        )
        .unwrap();

        let mut doc = config::load_doc(&path);
        doc["active"] = toml_edit::value("daylight");
        config::save_doc(&path, &doc).unwrap();

        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains("active = \"daylight\""));
        assert!(out.contains("# my shell config"));
        assert!(out.contains("[experimental]"));
        assert!(out.contains("foo = true"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
