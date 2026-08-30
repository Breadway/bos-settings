//! Shared helpers for the OS-panel commands: PATH lookups, tight name
//! checks, 0600 writes, and the Hyprland `source =` fragment convention.

use std::path::{Path, PathBuf};

use super::config;

/// Pacman packages these panels may install. A generic `pacman -S` runner
/// is an arbitrary-package primitive; every name must be on this list.
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
];

/// Bakery packages these panels may `bakery install`. breadcast is optional
/// software and is not on the ISO; do not add breadarr.
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
    std::process::Command::new("pacman")
        .args(["-Q", pkg])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Arch package / bakery name: starts alphanumeric, then `[A-Za-z0-9+._-]`.
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
        if !out.iter().any(|e| e == name) {
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

/// Pipe `input` to a command's stdin (`pkexec` does not inherit a piped
/// stdin unless we set it). Used by chpasswd and `pkexec tee`.
pub async fn run_with_stdin(args: &[&str], input: &str) -> bool {
    if args.is_empty() {
        return false;
    }
    let Ok(mut child) = tokio::process::Command::new(args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        if stdin.write_all(input.as_bytes()).await.is_err() {
            return false;
        }
    }
    child.wait().await.map(|s| s.success()).unwrap_or(false)
}

/// Atomic write with mode 0600 set on the new inode before/after replace,
/// matching breadcrumbs' `networks.toml` care.
pub fn write_secure(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }
    bread_utils::atomic::write_atomic(path, contents, Some(0o600))
        .map_err(|e| format!("writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex");
    re.replace_all(s, "").into_owned()
}

pub fn fail_output(output: &std::process::Output, what: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = stderr.trim();
    if !msg.is_empty() {
        return msg.to_string();
    }
    let msg = stdout.trim();
    if !msg.is_empty() {
        return msg.to_string();
    }
    format!("{what} failed")
}

pub fn hypr_dir() -> PathBuf {
    config::config_dir().join("hypr")
}

pub fn hyprland_conf() -> PathBuf {
    hypr_dir().join("hyprland.conf")
}

/// Ensure `hyprland.conf` sources `~/.config/hypr/{fragment}`. Appends a
/// single source line when missing; does not rewrite the rest of the file.
pub fn ensure_hypr_source(fragment: &str) -> Result<(), String> {
    if !valid_fragment(fragment) {
        return Err(format!("invalid hypr fragment '{fragment}'"));
    }
    let dir = hypr_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = hyprland_conf();
    let marker = format!("hypr/{fragment}");
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if existing.lines().any(|l| l.contains(&marker)) {
        return Ok(());
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("source = ~/.config/hypr/{fragment}\n"));
    config::atomic_write(&path, &text).map_err(|e| e.to_string())
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
    let filtered: String =
        existing
            .lines()
            .filter(|l| !l.contains(&marker))
            .fold(String::new(), |mut acc, l| {
                acc.push_str(l);
                acc.push('\n');
                acc
            });
    if filtered != existing {
        config::atomic_write(&path, &filtered).map_err(|e| e.to_string())?;
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

/// Connection / printer names: no flags, no newlines. Spaces are allowed
/// (NetworkManager connection ids often have them).
pub fn valid_nm_id(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t.len() <= 256
        && !t.starts_with('-')
        && !t.contains('\n')
        && !t.contains('\0')
        && !t.contains(';')
}

/// Reload Hyprland so settings.json / binds.json / monitors.json take effect now.
pub fn hypr_reload() {
    let _ = std::process::Command::new("hyprctl").arg("reload").status();
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

/// Linux account usernames: lowercase letters, digits, `_`, `-`, `.`;
/// must start with a lowercase letter (rejects a leading `-`, which would
/// be a flag injection into useradd/userdel); capped at useradd's 32-char MAX.
///
/// We deliberately do *not* accept a leading `@`/domain or spaces: account
/// creation here is a plain local user.
pub fn valid_username(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 32
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(*b, b'_' | b'-' | b'.'))
}

/// Free-form text thrown at a CLI tool as a single argv element (firewall
/// rule string, etc). Blocks the two genuinely dangerous shapes — a leading
/// flag `-` and any control/whitespace injection — while still allowing
/// spaces, slashes, dots, colons etc that ufw rules legitimately use.
pub fn valid_cli_value(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t.len() <= 256
        && !t.starts_with('-')
        && !t.contains('\n')
        && !t.contains('\r')
        && !t.contains('\0')
        && !t.contains(';')
        && t.bytes().all(|b| !(0..=31).contains(&b))
}

/// ufw / snapper numeric id — digits only.
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
        assert!(valid_pkg_name("libreoffice-fresh"));
        assert!(valid_pkg_name("nvidia-utils"));
    }

    #[test]
    fn pkg_name_rejects_flags() {
        assert!(!valid_pkg_name(""));
        assert!(!valid_pkg_name("-S"));
        assert!(!valid_pkg_name("--noconfirm"));
        assert!(!valid_pkg_name("foo;rm"));
        assert!(!valid_pkg_name("foo bar"));
    }

    #[test]
    fn allowlist_rejects_unknown() {
        assert!(allowed_pacman_packages(&["steam".into()]).is_ok());
        assert!(allowed_pacman_packages(&["evil".into()]).is_err());
        assert!(allowed_bakery_install("breadcast").is_ok());
        assert!(allowed_bakery_install("breadarr").is_err());
    }

    #[test]
    fn nm_id_allows_spaces_not_flags() {
        assert!(valid_nm_id("Home VPN"));
        assert!(!valid_nm_id("-evil"));
        assert!(!valid_nm_id("a\nb"));
        assert!(!valid_nm_id(""));
    }

    #[test]
    fn printer_name_is_tight() {
        assert!(valid_printer_name("Canon-TS6360a"));
        assert!(!valid_printer_name("foo bar"));
        assert!(!valid_printer_name("-d"));
    }

    #[test]
    fn username_accepts_normal_accounts() {
        assert!(valid_username("alice"));
        assert!(valid_username("bob_2"));
        assert!(valid_username("john.doe"));
        assert!(valid_username("a"));
    }

    #[test]
    fn username_rejects_flags_and_injection() {
        assert!(!valid_username(""));
        assert!(!valid_username("-r")); // system-account flag into useradd
        assert!(!valid_username("--system"));
        assert!(!valid_username("a\nb"));
        assert!(!valid_username("foo bar"));
        assert!(!valid_username("UPPER")); // must start lowercase
        assert!(!valid_username(&"a".repeat(33)));
    }

    #[test]
    fn cli_value_rejects_flags_and_controls() {
        assert!(valid_cli_value("80/tcp"));
        assert!(valid_cli_value("from 192.168.1.0/24 to any port 53"));
        assert!(!valid_cli_value("--all"));
        assert!(!valid_cli_value("-n"));
        assert!(!valid_cli_value("a\nb"));
        assert!(!valid_cli_value("a;rm"));
        assert!(!valid_cli_value(""));
    }

    #[test]
    fn number_id_is_digits_only() {
        assert!(valid_number_id("42"));
        assert!(!valid_number_id(""));
        assert!(!valid_number_id("-1"));
        assert!(!valid_number_id("12a"));
        assert!(!valid_number_id("1 2"));
    }
}
