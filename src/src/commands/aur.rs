//! AUR search via yay — graphical discovery beyond bakery's bread ecosystem
//! and [breadway]'s own republished packages.
//!
//! Installing opens a terminal running `yay -S <pkg>` instead of a silent
//! `--noconfirm` install — deliberate, not a shortcut skipped. AUR packages
//! run arbitrary maintainer-supplied build scripts, and yay's interactive
//! PKGBUILD diff review (plus the sudo prompt) is the actual safety
//! mechanism against a malicious/compromised package; automating it away
//! would remove the one step that exists to catch that.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct AurResult {
    name: String,
    version: String,
    description: String,
}

#[tauri::command]
pub async fn search_aur(query: String) -> Vec<AurResult> {
    let Ok(output) = tokio::process::Command::new("yay").args(["-Ss", "--aur", &query]).output().await else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(header) = lines.next() {
        // "aur/name version (+votes score) [Orphaned]" — name/version are
        // always the first two whitespace-separated fields after "aur/".
        let Some(rest) = header.strip_prefix("aur/") else { continue };
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let version = parts.next().unwrap_or("").to_string();
        let description = lines.next().unwrap_or("").trim().to_string();
        results.push(AurResult { name: name.to_string(), version, description });
        if results.len() >= 50 {
            break;
        }
    }
    results
}

#[tauri::command]
pub fn install_aur_package(pkg: String) {
    let _ = std::process::Command::new("kitty").args(["-e", "yay", "-S", &pkg]).spawn();
}
