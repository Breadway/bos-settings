//! breadpaper — wallpaper manager. No config file to edit here; breadpaper
//! takes no persistent settings, just an image path via its CLI
//! (`breadpaper set <path>` / `breadpaper get`). These commands are a thin
//! backend for that CLI so wallpaper (and the pywal-driven theme it
//! generates) has a discoverable home in Settings.

use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

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
        let Ok(entries) = std::fs::read_dir(dir) else { return };
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
pub async fn get_current_wallpaper() -> Option<String> {
    let out = Command::new("breadpaper").arg("get").output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
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
        Err("breadpaper failed — see terminal/journal".into())
    }
}

#[derive(Serialize)]
pub struct LibraryEntry {
    path: String,
    name: String,
}

/// Lists wallpapers under the library dir. Bounded/depth-limited (see the
/// constants above) — this is gated behind an explicit "Browse" click on
/// the frontend, not run at app launch, same "costs real time, gated
/// behind a button" posture as the network view's Wi-Fi scan.
#[tauri::command]
pub fn list_wallpaper_library() -> Vec<LibraryEntry> {
    scan_wallpapers(&wallpaper_library_dir())
        .into_iter()
        .map(|p| LibraryEntry {
            name: p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
            path: p.to_string_lossy().to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn wallpaper_library_dir_display() -> String {
    wallpaper_library_dir().to_string_lossy().to_string()
}
