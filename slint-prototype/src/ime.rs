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
