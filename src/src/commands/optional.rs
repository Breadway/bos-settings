//! Curated optional software. Not an AUR dump — four explicit offers,
//! each installed through a typed command (bakery or allowlisted pacman).

use serde::Serialize;
use tokio::process::Command;

use super::packages::get_installed_packages;
use super::util::{command_exists, fail_output, pacman_installed};

#[derive(Serialize, Clone)]
pub struct OptionalItem {
    id: String,
    title: String,
    detail: String,
    installed: bool,
    via: String,
}

#[derive(Serialize)]
pub struct OptionalStatus {
    items: Vec<OptionalItem>,
    flathub: bool,
}

fn bakery_has(name: &str) -> bool {
    get_installed_packages().iter().any(|p| p.name == name)
}

fn flathub_enabled() -> bool {
    if !command_exists("flatpak") {
        return false;
    }
    std::process::Command::new("flatpak")
        .args(["remotes"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_ascii_lowercase()
                .contains("flathub")
        })
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_optional_software() -> OptionalStatus {
    let breadcast = bakery_has("breadcast") || command_exists("breadcast");
    let flatpak = pacman_installed("flatpak") || command_exists("flatpak");
    let office = pacman_installed("libreoffice-fresh")
        && (pacman_installed("papers") || pacman_installed("evince"));
    let steam = pacman_installed("steam") || command_exists("steam");
    OptionalStatus {
        items: vec![
            OptionalItem {
                id: "breadcast".into(),
                title: "breadcast".into(),
                detail:
                    "Optional bread-ecosystem app. Installed through bakery — it is not on the ISO."
                        .into(),
                installed: breadcast,
                via: "bakery".into(),
            },
            OptionalItem {
                id: "flatpak".into(),
                title: "Flatpak + Flathub".into(),
                detail: "Enables the Flatpak runtime and the Flathub user remote.".into(),
                installed: flatpak && flathub_enabled(),
                via: "pacman".into(),
            },
            OptionalItem {
                id: "office".into(),
                title: "LibreOffice + PDF".into(),
                detail: "libreoffice-fresh and papers (GNOME document viewer).".into(),
                installed: office,
                via: "pacman".into(),
            },
            OptionalItem {
                id: "steam".into(),
                title: "Steam".into(),
                detail: "Valve Steam from the multilib repo.".into(),
                installed: steam,
                via: "pacman".into(),
            },
        ],
        flathub: flathub_enabled(),
    }
}

/// User Flathub remote — no root. Flatpak itself is installed separately
/// via the allowlisted pacman command when missing.
#[tauri::command]
pub async fn enable_flathub() -> Result<(), String> {
    if !command_exists("flatpak") {
        return Err("flatpak is not installed".into());
    }
    let output = Command::new("flatpak")
        .args([
            "remote-add",
            "--if-not-exists",
            "--user",
            "flathub",
            "https://dl.flathub.org/repo/flathub.flatpakrepo",
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(fail_output(&output, "flatpak remote-add"))
    }
}
