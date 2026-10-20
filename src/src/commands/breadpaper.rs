//! breadpaper: wallpaper manager. No config file to edit here; breadpaper
//! takes no persistent settings, just an image path via its CLI
//! (`breadpaper set <path>` / `breadpaper get`). These commands are a thin
//! backend for that CLI so wallpaper (and the wallpaper-derived theme it
//! generates) has a discoverable home in Settings.

use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;
use tokio::process::Command;

/// The webview previews wallpapers through the asset protocol, whose static
/// scope only covers the library and the system wallpaper folders. A current
/// wallpaper anywhere else (BSET-70) is allowed here as that single file, and
/// only when it really is an image file that exists.
fn allow_preview(app: &tauri::AppHandle, path: &str) {
    if let Some(resolved) = preview_target(Path::new(path)) {
        let _ = app.asset_protocol_scope().allow_file(&resolved);
    }
}

/// The file that may be added to the asset scope for `path`, or `None`.
///
/// The path is resolved with `canonicalize` first and every check (absolute,
/// image extension, is a regular file) runs on the resolved path. Checking the
/// raw path would let a symlink named `*.png` point the scope at a non-image
/// target. Returning the resolved path also means the scope grants exactly
/// the file the checks ran against.
fn preview_target(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let resolved = std::fs::canonicalize(path).ok()?;
    if is_wallpaper_file(&resolved) && resolved.is_file() {
        Some(resolved)
    } else {
        None
    }
}

/// Extensions breadpaper's own `validate()` accepts.
const WALLPAPER_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Caps how many thumbnails the library ever returns, and how deep the
/// recursive scan goes (the library is organized in subfolders, e.g. by
/// show/series, so a non-recursive scan would find nothing).
const MAX_LIBRARY_ITEMS: usize = 80;
const MAX_SCAN_DEPTH: usize = 4;

pub fn wallpaper_library_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join("Pictures/Backgrounds")
}

fn is_wallpaper_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| WALLPAPER_EXTS.iter().any(|ext| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

fn scan_wallpapers(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth == 0 || out.len() >= MAX_LIBRARY_ITEMS {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if out.len() >= MAX_LIBRARY_ITEMS {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth - 1, out);
            } else if is_wallpaper_file(&path) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, MAX_SCAN_DEPTH, &mut out);
    out
}

#[tauri::command]
pub async fn get_current_wallpaper(app: tauri::AppHandle) -> Option<String> {
    let out = Command::new("breadpaper").arg("get").output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        allow_preview(&app, &s);
        Some(s)
    }
}

#[tauri::command]
pub async fn set_wallpaper(path: String) -> Result<(), String> {
    let ok = Command::new("breadpaper")
        .arg("set")
        .arg(&path)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("breadpaper failed: see terminal/journal".into())
    }
}

/// One compositor output's persisted wallpaper, from breadpaper's own
/// `~/.config/breadpaper/current.json` (`{ "outputs": { "<name>": "<path>" } }`).
#[derive(Serialize)]
pub struct OutputWallpaper {
    output: String,
    path: String,
}

fn current_json_path() -> PathBuf {
    super::config::config_dir().join("breadpaper/current.json")
}

/// The wallpaper breadpaper last set on each output. Read straight from
/// `current.json` rather than shelling `breadpaper --output <n> get` once
/// per monitor; it's the same file breadpaper's `get_on` reads, and a
/// settings window can have several monitors to report.
#[tauri::command]
pub fn get_wallpapers_by_output(app: tauri::AppHandle) -> Vec<OutputWallpaper> {
    let Ok(text) = std::fs::read_to_string(current_json_path()) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(outputs) = value.get("outputs").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    outputs
        .iter()
        .filter_map(|(output, path)| {
            let path = path.as_str()?.to_string();
            allow_preview(&app, &path);
            Some(OutputWallpaper {
                output: output.clone(),
                path,
            })
        })
        .collect()
}

/// Set the wallpaper (and regenerate that output's palette) on a
/// single compositor output. `breadpaper --output <NAME> set <path>`
/// leaves every other output's wallpaper untouched (see breadpaper's
/// `current.json` `set_one_output_does_not_drop_others` test).
#[tauri::command]
pub async fn set_wallpaper_on(output: String, path: String) -> Result<(), String> {
    let ok = Command::new("breadpaper")
        .args(["--output", &output, "set"])
        .arg(&path)
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err("breadpaper failed: see terminal/journal".into())
    }
}

#[derive(Serialize)]
pub struct LibraryEntry {
    path: String,
    name: String,
}

/// Lists wallpapers under the library dir. Bounded/depth-limited (see the
/// constants above); this is gated behind an explicit "Browse" click on
/// the frontend, not run at app launch, same "costs real time, gated
/// behind a button" posture as the network view's Wi-Fi scan.
#[tauri::command]
pub fn list_wallpaper_library() -> Vec<LibraryEntry> {
    scan_wallpapers(&wallpaper_library_dir())
        .into_iter()
        .map(|p| LibraryEntry {
            name: p
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: p.to_string_lossy().to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn wallpaper_library_dir_display() -> String {
    wallpaper_library_dir().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh scratch directory per test, cleaned up on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "bos-settings-breadpaper-test-{}-{name}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn normal_image_file_is_allowed() {
        let dir = Scratch::new("normal");
        let img = dir.path().join("wall.png");
        std::fs::write(&img, b"not really a png").unwrap();

        let got = preview_target(&img).expect("an existing image should be allowed");
        assert_eq!(got, std::fs::canonicalize(&img).unwrap());
    }

    #[test]
    fn missing_file_is_refused() {
        let dir = Scratch::new("missing");
        let img = dir.path().join("gone.png");

        assert_eq!(preview_target(&img), None);
    }

    #[test]
    fn symlink_to_a_non_image_is_refused() {
        let dir = Scratch::new("symlink-nonimage");
        let secret = dir.path().join("secret.txt");
        std::fs::write(&secret, b"not an image").unwrap();
        // Named like a wallpaper, but the resolved target is not an image.
        let link = dir.path().join("wall.png");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        assert_eq!(preview_target(&link), None);
    }

    #[test]
    fn symlink_named_png_to_real_image_resolves_to_target() {
        let dir = Scratch::new("symlink-image");
        let real = dir.path().join("real.jpg");
        std::fs::write(&real, b"pixels").unwrap();
        let link = dir.path().join("wall.png");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // The scope is granted the resolved file, not the link name.
        let got = preview_target(&link).expect("symlink to an image should resolve");
        assert_eq!(got, std::fs::canonicalize(&real).unwrap());
    }

    #[test]
    fn relative_path_is_refused() {
        assert_eq!(preview_target(Path::new("wall.png")), None);
    }

    #[test]
    fn directory_named_like_an_image_is_refused() {
        let dir = Scratch::new("dir");
        let d = dir.path().join("folder.png");
        std::fs::create_dir_all(&d).unwrap();

        assert_eq!(preview_target(&d), None);
    }
}
