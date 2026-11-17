//! Shared helpers: PATH lookups, tight name checks, 0600 writes and the
//! Hyprland `source =` fragment convention.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::config;

/// Pacman packages these panels may install. A generic `pacman -S` runner is
/// an arbitrary-package primitive; every name must be on this list.
pub const PACMAN_ALLOWLIST: &[&str] = &[
    "hyprsunset",
    "fcitx5",
    "fcitx5-configtool",
    "fcitx5-gtk",
    "fcitx5-qt",
    "fcitx5-im",
    "fcitx5-chinese-addons",
    "fcitx5-table-extra",
    "orca",
    "kmag",
    "restic",
    "flatpak",
    "libreoffice-fresh",
    "papers",
    "evince",
    "steam",
    "nvidia",
    "nvidia-utils",
    "tlp-pd",
    "kdeconnect",
];

pub const BAKERY_INSTALL_ALLOWLIST: &[&str] = &["breadcast"];

pub fn command_exists(name: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(name);
        candidate.is_file()
    })
}

pub fn pacman_installed(pkg: &str) -> bool {
    Command::new("pacman")
        .args(["-Q", pkg])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn valid_pkg_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_' | b'+' | b'.'))
}

pub fn allowed_pacman_packages(names: &[String]) -> Result<Vec<String>, String> {
    if names.is_empty() {
        return Err("no packages given".into());
    }
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        if !valid_pkg_name(name) || !PACMAN_ALLOWLIST.contains(&name.as_str()) {
            return Err(format!("refusing to install '{name}'"));
        }
        if !out.iter().any(|entry| entry == name) {
            out.push(name.clone());
        }
    }
    Ok(out)
}

pub fn allowed_bakery_install(name: &str) -> Result<(), String> {
    if !valid_pkg_name(name) || !BAKERY_INSTALL_ALLOWLIST.contains(&name) {
        return Err(format!("refusing to bakery-install '{name}'"));
    }
    Ok(())
}

pub fn bos_settings_dir() -> PathBuf {
    config::config_dir().join("bos-settings")
}

/// Pipe `input` to a command's stdin, the way chpasswd and `tee` are fed.
pub fn run_with_stdin(args: &[&str], input: &str) -> bool {
    if args.is_empty() {
        return false;
    }
    let Ok(mut child) = Command::new(args[0])
        .args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        if stdin.write_all(input.as_bytes()).is_err() {
            return false;
        }
    }
    child.wait().map(|status| status.success()).unwrap_or(false)
}

/// Atomic write with mode 0600 set on the new file, matching breadcrumbs'
/// `networks.toml` care.
pub fn write_secure(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }
    let temp = path.with_extension("tmp-secure");
    std::fs::write(&temp, contents)
        .map_err(|error| format!("writing {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("securing {}: {error}", temp.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|error| format!("writing {}: {error}", path.display()))?;
    Ok(())
}

pub fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex");
    re.replace_all(s, "").into_owned()
}

pub fn fail_output(output: &Output, what: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = stderr.trim();
    if !message.is_empty() {
        return message.to_string();
    }
    let message = stdout.trim();
    if !message.is_empty() {
        return message.to_string();
    }
    format!("{what} failed")
}

pub fn hypr_dir() -> PathBuf {
    config::config_dir().join("hypr")
}

pub fn hyprland_conf() -> PathBuf {
    hypr_dir().join("hyprland.conf")
}

pub fn ensure_hypr_source(fragment: &str) -> Result<(), String> {
    if !valid_fragment(fragment) {
        return Err(format!("invalid hypr fragment '{fragment}'"));
    }
    let dir = hypr_dir();
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = hyprland_conf();
    let marker = format!("hypr/{fragment}");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if existing.lines().any(|line| line.contains(&marker)) {
        return Ok(());
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("source = ~/.config/hypr/{fragment}\n"));
    config::atomic_write(&path, &text).map_err(|error| error.to_string())
}

pub fn remove_hypr_source(fragment: &str) -> Result<(), String> {
    if !valid_fragment(fragment) {
        return Err(format!("invalid hypr fragment '{fragment}'"));
    }
    let path = hyprland_conf();
    let Ok(existing) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let marker = format!("hypr/{fragment}");
    let filtered: String = existing
        .lines()
        .filter(|line| !line.contains(&marker))
        .fold(String::new(), |mut acc, line| {
            acc.push_str(line);
            acc.push('\n');
            acc
        });
    if filtered != existing {
        config::atomic_write(&path, &filtered).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn valid_fragment(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_' | b'.'))
}

/// Connection and printer names: no flags, no newlines. Spaces are allowed.
pub fn valid_nm_id(name: &str) -> bool {
    let text = name.trim();
    !text.is_empty()
        && text.len() <= 256
        && !text.starts_with('-')
        && !text.contains('\n')
        && !text.contains('\0')
        && !text.contains(';')
}

pub fn hypr_reload() {
    let _ = Command::new("hyprctl").arg("reload").status();
}

pub fn valid_printer_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 127
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_' | b'.'))
}

/// ufw and snapper numeric ids: digits only.
pub fn valid_number_id(num: &str) -> bool {
    !num.is_empty() && num.len() <= 12 && num.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkg_name_accepts_arch_names() {
        assert!(valid_pkg_name("hyprsunset"));
        assert!(valid_pkg_name("fcitx5-chinese-addons"));
        assert!(!valid_pkg_name("--noconfirm"));
    }

    #[test]
    fn allowlist_rejects_unknown() {
        assert!(allowed_pacman_packages(&["steam".into()]).is_ok());
        assert!(allowed_pacman_packages(&["evil".into()]).is_err());
        assert!(allowed_bakery_install("breadcast").is_ok());
        assert!(allowed_bakery_install("breadarr").is_err());
    }

    #[test]
    fn names_stay_tight() {
        assert!(valid_nm_id("Home VPN"));
        assert!(!valid_nm_id("-evil"));
        assert!(valid_printer_name("Canon-TS6360a"));
        assert!(!valid_printer_name("foo bar"));
        assert!(valid_number_id("42"));
        assert!(!valid_number_id("12a"));
    }

    #[test]
    fn ansi_escapes_are_removed() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }
}
