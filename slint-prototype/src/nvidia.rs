//! NVIDIA driver offer. BOS writes a probe file when it sees a discrete NVIDIA
//! GPU; Settings only shows the card if that file exists and installs nothing
//! until the user clicks.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const NVIDIA_LUA: &str = "\
-- Written by bos-nvidia-setup. hyprland.lua dofiles this only when it exists.
-- Hyprland 0.56 (Aquamarine): no WLR_* variables.
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

pub const NVIDIA_PACKAGES: [&str; 2] = ["nvidia", "nvidia-utils"];

#[derive(Clone)]
pub struct Offer {
    pub gpu: String,
    pub reason: String,
    pub packages: Vec<String>,
    pub installed: bool,
}

fn offer_paths() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let state = Path::new(&home).join(".local/state/bos");
    vec![
        state.join("nvidia-offer.json"),
        state.join("nvidia-probe.json"),
    ]
}

fn dropin_path() -> PathBuf {
    crate::util::hypr_dir().join("nvidia.lua")
}

fn nvidia_ready() -> bool {
    dropin_path().is_file()
        && NVIDIA_PACKAGES
            .iter()
            .all(|package| crate::util::pacman_installed(package))
}

fn default_packages() -> Vec<String> {
    NVIDIA_PACKAGES.iter().map(|name| (*name).to_string()).collect()
}

fn generic_offer() -> Offer {
    Offer {
        gpu: "NVIDIA GPU".into(),
        reason: "BOS found an NVIDIA device. Install the proprietary driver only if you want it; nouveau stays otherwise.".into(),
        packages: default_packages(),
        installed: nvidia_ready(),
    }
}

pub fn read_offer() -> Option<Offer> {
    for path in offer_paths() {
        if !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Some(generic_offer());
        };
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if value.get("offer").and_then(|flag| flag.as_bool()) == Some(false)
                || value.get("dismissed").and_then(|flag| flag.as_bool()) == Some(true)
            {
                return None;
            }
            let gpu = value
                .get("gpu")
                .or_else(|| value.get("name"))
                .or_else(|| value.get("device"))
                .and_then(|item| item.as_str())
                .unwrap_or("NVIDIA GPU")
                .to_string();
            let reason = value
                .get("reason")
                .or_else(|| value.get("message"))
                .and_then(|item| item.as_str())
                .unwrap_or("A discrete NVIDIA GPU was detected. The proprietary driver is not installed until you choose it.")
                .to_string();
            let packages = value
                .get("packages")
                .and_then(|item| item.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|row| row.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .filter(|packages| !packages.is_empty())
                .unwrap_or_else(default_packages);
            return Some(Offer {
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

fn write_dropin() -> Result<(), String> {
    let dir = crate::util::hypr_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = dir.join("nvidia.lua");
    crate::config::atomic_write(&path, NVIDIA_LUA).map_err(|error| error.to_string())?;
    ensure_hyprland_include()
}

fn include_already_present(text: &str) -> bool {
    text.contains("nvidia.lua")
}

fn ensure_hyprland_include() -> Result<(), String> {
    let path = crate::util::hypr_dir().join("hyprland.lua");
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
    crate::config::atomic_write(&path, &text).map_err(|error| error.to_string())
}

/// Install nvidia + nvidia-utils and write the Hyprland env drop-in. Prefers
/// the ISO's `bos-nvidia-setup`; falls back to the allowlisted package
/// installer plus the same drop-in.
pub fn setup() -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_default();
    if crate::util::command_exists("bos-nvidia-setup") {
        return Command::new("pkexec")
            .args(["bos-nvidia-setup", "--home", &home])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    crate::updates::pacman_install(&default_packages())?;
    write_dropin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropin_has_current_wiki_env() {
        assert!(NVIDIA_LUA.contains("LIBVA_DRIVER_NAME"));
        assert!(NVIDIA_LUA.contains("NVD_BACKEND"));
        assert!(!NVIDIA_LUA.contains("hl.env(\"WLR_"));
    }

    #[test]
    fn include_is_conditional() {
        assert!(HYPR_INCLUDE.contains("io.open"));
        assert!(include_already_present("pcall(dofile, nvidia.lua)"));
        assert!(!include_already_present("hl.env(1, 2)"));
    }
}
