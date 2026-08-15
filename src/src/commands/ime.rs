//! fcitx5 input method for this session: environment.d + Hyprland env +
//! systemd --user / `fcitx5 -d`. Missing packages are offered via the
//! allowlisted pacman installer, not installed on page load.

use serde::Serialize;
use tokio::process::Command;

use super::config;
use super::util::{self, command_exists, pacman_installed};

const FRAGMENT: &str = "fcitx5.conf";
const ENV_FILE: &str = "90-fcitx5.conf";

const ENV_LINES_SYSTEMD: &str = "\
GTK_IM_MODULE=fcitx
QT_IM_MODULE=fcitx
XMODIFIERS=@im=fcitx
SDL_IM_MODULE=fcitx
";

const ENV_LINES_HYPR: &str = "\
env = GTK_IM_MODULE,fcitx
env = QT_IM_MODULE,fcitx
env = XMODIFIERS,@im=fcitx
env = SDL_IM_MODULE,fcitx
exec-once = fcitx5 -d
";

#[derive(Serialize, Clone)]
pub struct ImePackage {
    name: String,
    installed: bool,
}

#[derive(Serialize)]
pub struct ImeStatus {
    enabled: bool,
    running: bool,
    packages: Vec<ImePackage>,
    error: Option<String>,
}

fn env_path() -> std::path::PathBuf {
    config::config_dir().join("environment.d").join(ENV_FILE)
}

fn wanted_packages() -> &'static [&'static str] {
    &[
        "fcitx5",
        "fcitx5-gtk",
        "fcitx5-qt",
        "fcitx5-configtool",
        "fcitx5-chinese-addons",
    ]
}

fn packages_status() -> Vec<ImePackage> {
    wanted_packages()
        .iter()
        .map(|name| ImePackage {
            name: (*name).to_string(),
            installed: pacman_installed(name),
        })
        .collect()
}

fn env_file_present() -> bool {
    env_path().is_file()
}

async fn fcitx_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "fcitx5"])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn get_ime_status() -> ImeStatus {
    ImeStatus {
        enabled: env_file_present(),
        running: fcitx_running().await,
        packages: packages_status(),
        error: None,
    }
}

#[tauri::command]
pub async fn set_ime_enabled(enabled: bool) -> Result<ImeStatus, String> {
    if enabled {
        enable_ime().await?;
    } else {
        disable_ime().await?;
    }
    Ok(ImeStatus {
        enabled: env_file_present(),
        running: fcitx_running().await,
        packages: packages_status(),
        error: None,
    })
}

async fn enable_ime() -> Result<(), String> {
    if !command_exists("fcitx5") {
        return Err("fcitx5 is not installed".into());
    }
    let env = env_path();
    if let Some(parent) = env.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    config::atomic_write(&env, ENV_LINES_SYSTEMD).map_err(|e| e.to_string())?;

    let hypr = util::hypr_dir().join(FRAGMENT);
    std::fs::create_dir_all(util::hypr_dir()).map_err(|e| e.to_string())?;
    config::atomic_write(&hypr, ENV_LINES_HYPR).map_err(|e| e.to_string())?;
    util::ensure_hypr_source(FRAGMENT)?;

    let _ = Command::new("systemctl")
        .args([
            "--user",
            "import-environment",
            "GTK_IM_MODULE",
            "QT_IM_MODULE",
            "XMODIFIERS",
            "SDL_IM_MODULE",
        ])
        .status()
        .await;

    let enabled_unit = Command::new("systemctl")
        .args(["--user", "enable", "--now", "fcitx5.service"])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);
    if !enabled_unit && !fcitx_running().await {
        std::process::Command::new("fcitx5")
            .arg("-d")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("couldn't start fcitx5: {e}"))?;
    }
    Ok(())
}

async fn disable_ime() -> Result<(), String> {
    let _ = std::fs::remove_file(env_path());
    let _ = std::fs::remove_file(util::hypr_dir().join(FRAGMENT));
    util::remove_hypr_source(FRAGMENT)?;
    let _ = Command::new("systemctl")
        .args(["--user", "disable", "--now", "fcitx5.service"])
        .status()
        .await;
    let _ = Command::new("pkill").args(["-x", "fcitx5"]).status().await;
    Ok(())
}

#[tauri::command]
pub fn open_fcitx_config() {
    let _ = std::process::Command::new("fcitx5-configtool").spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_files_use_fcitx_module_name() {
        assert!(ENV_LINES_SYSTEMD.contains("GTK_IM_MODULE=fcitx"));
        assert!(ENV_LINES_HYPR.contains("XMODIFIERS,@im=fcitx"));
        assert!(ENV_LINES_HYPR.contains("exec-once = fcitx5 -d"));
    }
}
