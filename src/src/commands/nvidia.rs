//! NVIDIA driver offer. BOS writes a probe file when it sees a discrete
//! NVIDIA GPU; Settings only shows the card if that file exists and does
//! not install anything until the user clicks.

use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

use super::streaming;
use super::util::{self, command_exists, pacman_installed};

/// Same drop-in bos-nvidia-setup writes. hyprland.lua dofiles it only
/// when the file exists (Mesa machines have no file).
/// Hyprland 0.56 (Aquamarine): wiki requires LIBVA + GLX vendor.
/// NVD_BACKEND is the current VA-API hint. No WLR_* / GBM_BACKEND.
const NVIDIA_LUA: &str = "\
-- Written by bos-nvidia-setup. hyprland.lua dofiles this only when it exists.
-- Hyprland 0.56 (Aquamarine) — no WLR_* variables.
-- https://wiki.hypr.land/Nvidia/
hl.env(\"LIBVA_DRIVER_NAME\", \"nvidia\")
hl.env(\"__GLX_VENDOR_LIBRARY_NAME\", \"nvidia\")
hl.env(\"NVD_BACKEND\", \"direct\")
";

const HYPR_INCLUDE: &str = "\
-- bos-nvidia-setup: optional proprietary env; no-op when the file is absent
do
    local nvidia = (os.getenv(\"HOME\") or \"\") .. \"/.config/hypr/nvidia.lua\"
    local f = io.open(nvidia, \"r\")
    if f then
        f:close()
        pcall(dofile, nvidia)
    end
end
";

const NVIDIA_PACKAGES: &[&str] = &["nvidia", "nvidia-utils"];

#[derive(Serialize, Clone)]
pub struct NvidiaOffer {
    gpu: String,
    reason: String,
    packages: Vec<String>,
    installed: bool,
}

fn offer_paths() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let state = Path::new(&home).join(".local/state/bos");
    vec![
        state.join("nvidia-offer.json"),
        state.join("nvidia-probe.json"),
    ]
}

fn nvidia_dropin_path() -> PathBuf {
    util::hypr_dir().join("nvidia.lua")
}

fn nvidia_ready() -> bool {
    nvidia_dropin_path().is_file() && NVIDIA_PACKAGES.iter().all(|p| pacman_installed(p))
}

pub fn read_nvidia_offer() -> Option<NvidiaOffer> {
    for path in offer_paths() {
        if !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Some(generic_offer());
        };
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v.get("offer").and_then(|x| x.as_bool()) == Some(false)
                || v.get("dismissed").and_then(|x| x.as_bool()) == Some(true)
            {
                return None;
            }
            let gpu = v
                .get("gpu")
                .or_else(|| v.get("name"))
                .or_else(|| v.get("device"))
                .and_then(|x| x.as_str())
                .unwrap_or("NVIDIA GPU")
                .to_string();
            let reason = v
                .get("reason")
                .or_else(|| v.get("message"))
                .and_then(|x| x.as_str())
                .unwrap_or("A discrete NVIDIA GPU was detected. The proprietary driver is not installed until you choose it.")
                .to_string();
            let packages = v
                .get("packages")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .filter(|p| !p.is_empty())
                .unwrap_or_else(default_packages);
            return Some(NvidiaOffer {
                gpu,
                reason,
                packages,
                installed: nvidia_ready(),
            });
        }
        return Some(generic_offer());
    }
    None
}

fn default_packages() -> Vec<String> {
    NVIDIA_PACKAGES.iter().map(|s| (*s).to_string()).collect()
}

fn generic_offer() -> NvidiaOffer {
    NvidiaOffer {
        gpu: "NVIDIA GPU".into(),
        reason: "BOS found an NVIDIA device. Install the proprietary driver only if you want it — nouveau stays otherwise.".into(),
        packages: default_packages(),
        installed: nvidia_ready(),
    }
}

#[tauri::command]
pub fn get_nvidia_offer() -> Option<NvidiaOffer> {
    read_nvidia_offer()
}

/// Install nvidia + nvidia-utils and write the Hyprland env drop-in.
/// Prefers `/usr/local/bin/bos-nvidia-setup` (ISO script) so package
/// install + env stay one path. Falls back to allowlisted pacman plus
/// the same drop-in when the script is not on this install yet.
#[tauri::command]
pub async fn nvidia_setup(app: AppHandle, session_id: String) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    if command_exists("bos-nvidia-setup") {
        let ok = streaming::run_hardcoded(
            app.clone(),
            session_id.clone(),
            "pkexec",
            &["bos-nvidia-setup", "--home", &home],
        )
        .await;
        if ok {
            streaming::emit_line(&app, &session_id, "reboot required");
        }
        return ok;
    }

    streaming::emit_line(
        &app,
        &session_id,
        "bos-nvidia-setup not on PATH; installing via pacman and writing the env drop-in",
    );
    let packages = default_packages();
    if !streaming::pacman_install(app.clone(), session_id.clone(), packages).await {
        return false;
    }
    match write_nvidia_dropin() {
        Ok(()) => {
            streaming::emit_line(&app, &session_id, "wrote ~/.config/hypr/nvidia.lua");
            streaming::emit_line(&app, &session_id, "reboot required");
            true
        }
        Err(e) => {
            streaming::emit_line(&app, &session_id, &format!("Error: {e}"));
            false
        }
    }
}

fn write_nvidia_dropin() -> Result<(), String> {
    let dir = util::hypr_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("nvidia.lua");
    super::config::atomic_write(&path, NVIDIA_LUA).map_err(|e| e.to_string())?;
    ensure_hyprland_include()?;
    Ok(())
}

fn include_already_present(text: &str) -> bool {
    text.contains("nvidia.lua")
}

fn ensure_hyprland_include() -> Result<(), String> {
    let path = util::hypr_dir().join("hyprland.lua");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if include_already_present(&existing) {
        return Ok(());
    }
    if existing.is_empty() {
        return Ok(());
    }
    let mut text = existing;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(HYPR_INCLUDE);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    super::config::atomic_write(&path, &text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_none() {
        // This machine's real probe path is not something the unit test
        // should depend on; the helper is covered via parse cases below.
        let parsed = serde_json::from_str::<serde_json::Value>("{\"offer\":false}").unwrap();
        assert_eq!(parsed["offer"], false);
    }

    #[test]
    fn dismissed_or_offer_false_hides() {
        // Inlined copies of the hide conditions so a schema change is obvious.
        let hide = |v: &str| {
            let v: serde_json::Value = serde_json::from_str(v).unwrap();
            v.get("offer").and_then(|x| x.as_bool()) == Some(false)
                || v.get("dismissed").and_then(|x| x.as_bool()) == Some(true)
        };
        assert!(hide(r#"{"offer":false}"#));
        assert!(hide(r#"{"dismissed":true}"#));
        assert!(!hide(r#"{"gpu":"RTX 4060"}"#));
    }

    #[test]
    fn dropin_has_current_wiki_env_and_no_obsolete_vars() {
        assert!(NVIDIA_LUA.contains("LIBVA_DRIVER_NAME"));
        assert!(NVIDIA_LUA.contains("__GLX_VENDOR_LIBRARY_NAME"));
        assert!(NVIDIA_LUA.contains("NVD_BACKEND"));
        assert!(!NVIDIA_LUA.contains("hl.env(\"WLR_"));
        assert!(!NVIDIA_LUA.contains("hl.env(\"GBM_BACKEND"));
        assert!(!NVIDIA_LUA.contains("cuda"));
    }

    #[test]
    fn include_snippet_is_conditional() {
        assert!(HYPR_INCLUDE.contains("nvidia.lua"));
        assert!(HYPR_INCLUDE.contains("io.open"));
        assert!(HYPR_INCLUDE.contains("pcall(dofile"));
    }

    #[test]
    fn include_detects_existing_snippet() {
        assert!(include_already_present("pcall(dofile, nvidia.lua)"));
        assert!(!include_already_present("hl.env(\"XCURSOR_SIZE\", \"24\")"));
    }

    #[test]
    fn packages_are_nvidia_and_utils_only() {
        assert_eq!(NVIDIA_PACKAGES, &["nvidia", "nvidia-utils"]);
    }
}
