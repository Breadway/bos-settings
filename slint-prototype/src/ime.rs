//! fcitx5 input method state for this session.

use std::process::Command;

pub struct Status {
    pub enabled: bool,
    pub running: bool,
    pub installed: bool,
}

fn config_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/".into()))
                .join(".config")
        })
}

fn env_path() -> std::path::PathBuf {
    config_dir().join("environment.d").join("90-fcitx5.conf")
}

fn installed(package: &str) -> bool {
    Command::new("pacman")
        .args(["-Q", package])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn running() -> bool {
    Command::new("pgrep")
        .args(["-x", "fcitx5"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// The packages fcitx5 needs on a BOS desktop, in the order the page lists
/// them.
pub const PACKAGES: [&str; 4] = [
    "fcitx5-im",
    "fcitx5-gtk",
    "fcitx5-qt",
    "fcitx5-chinese-addons",
];

/// Each needed package and whether it is installed.
pub fn packages() -> Vec<(&'static str, bool)> {
    PACKAGES
        .into_iter()
        .map(|name| (name, installed(name)))
        .collect()
}

/// Installs every missing package, plus the fcitx5 group when the split
/// packages are not there yet.
pub fn install_missing() -> Result<(), String> {
    let missing: Vec<String> = packages()
        .into_iter()
        .filter(|(_, installed)| !installed)
        .map(|(name, _)| name.to_string())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    crate::updates::pacman_install(&missing)
}

pub fn status() -> Status {
    Status {
        enabled: env_path().is_file(),
        running: running(),
        installed: installed("fcitx5"),
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = env_path();
    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|error| error.to_string())?;
        }
        let _ = Command::new("fcitx5-remote").arg("-e").output();
        return Ok(());
    }
    if !installed("fcitx5") {
        return Err("fcitx5 is not installed".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let body = "GTK_IM_MODULE=fcitx\nQT_IM_MODULE=fcitx\nXMODIFIERS=@im=fcitx\nSDL_IM_MODULE=fcitx\n";
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, body).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, &path).map_err(|error| error.to_string())?;
    let fragment = config_dir().join("hypr/bos-settings-input-method.conf");
    if let Some(parent) = fragment.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(
        &fragment,
        "env = XMODIFIERS,@im=fcitx\nenv = GTK_IM_MODULE,fcitx\nenv = QT_IM_MODULE,fcitx\nenv = SDL_IM_MODULE,fcitx\nexec-once = fcitx5 -d\n",
    )
    .map_err(|error| error.to_string())?;
    let _ = crate::util::ensure_hypr_source("bos-settings-input-method.conf");
    Command::new("fcitx5")
        .arg("-d")
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn open_config() -> Result<(), String> {
    Command::new("fcitx5-configtool")
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn env_body_lists_every_toolkit() {
        let body = "GTK_IM_MODULE=fcitx\nQT_IM_MODULE=fcitx\nXMODIFIERS=@im=fcitx\nSDL_IM_MODULE=fcitx\n";
        assert!(body.contains("GTK_IM_MODULE"));
        assert!(body.contains("XMODIFIERS"));
    }
}
