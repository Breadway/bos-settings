//! File backups through restic, plus snapper snapshots.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub repo: String,
    pub has_password: bool,
    password: String,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub number: String,
    pub date: String,
    pub description: String,
}

#[derive(Clone, Debug)]
pub struct ResticSnapshot {
    pub id: String,
    pub time: String,
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn dir_path() -> PathBuf {
    config_dir().join("bos-settings")
}

pub fn config_path() -> PathBuf {
    dir_path().join("backup.toml")
}

pub fn load() -> Config {
    let Ok(text) = std::fs::read_to_string(config_path()) else {
        return Config::default();
    };
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    let repo = doc
        .get("repo")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    let password = doc
        .get("password")
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    Config {
        repo,
        has_password: !password.is_empty(),
        password,
    }
}

pub fn save(repo: &str, password: Option<&str>) -> Result<(), String> {
    if !valid_repo(repo) {
        return Err("repo must be an absolute path or sftp:user@host:path".into());
    }
    let existing = load();
    let secret = password
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            if existing.password.is_empty() {
                None
            } else {
                Some(existing.password)
            }
        });
    let mut doc = toml_edit::DocumentMut::new();
    doc.insert("repo", toml_edit::value(repo.trim()));
    if let Some(secret) = secret {
        doc.insert("password", toml_edit::value(secret));
    }
    let path = config_path();
    let Some(parent) = path.parent() else {
        return Err("config path has no parent".into());
    };
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(&path, doc.to_string()).map_err(|error| error.to_string())?;
    let saved = load();
    if !saved.password.is_empty() {
        let pass = dir_path().join("restic.pass");
        std::fs::write(&pass, &saved.password).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&pass, std::fs::Permissions::from_mode(0o600));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn valid_repo(repo: &str) -> bool {
    let repo = repo.trim();
    if repo.is_empty() || repo.len() > 512 || repo.contains('\n') || repo.contains('\0') {
        return false;
    }
    if let Some(rest) = repo.strip_prefix("sftp:") {
        return !rest.is_empty() && rest.contains('@') && rest.contains(':') && !rest.contains(' ');
    }
    Path::new(repo).is_absolute()
}

pub fn valid_restore_target(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        && !path.to_string_lossy().contains('\n')
        && !path.to_string_lossy().contains('\0')
}

pub fn restic_snapshots() -> Result<Vec<ResticSnapshot>, String> {
    let config = load();
    if !valid_repo(&config.repo) || !config.has_password {
        return Err("set a repository and password first".into());
    }
    let out = Command::new("restic")
        .args(["--repo", &config.repo, "snapshots", "--json"])
        .env("RESTIC_PASSWORD", &config.password)
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let Ok(rows) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return Err("restic returned unreadable output".into());
    };
    Ok(rows
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some(ResticSnapshot {
                        id: row.get("short_id")?.as_str()?.to_string(),
                        time: row.get("time")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

fn run_restic(action: &str, args: &[String], password: &str) -> Result<(), String> {
    let dir = config_dir().join("bos-settings");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let pass = dir.join("restic.pass");
    std::fs::write(&pass, password).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&pass, std::fs::Permissions::from_mode(0o600));
    }
    Command::new("kitty")
        .arg("-e")
        .arg("env")
        .arg(format!("RESTIC_PASSWORD_FILE={}", pass.display()))
        .arg("restic")
        .arg("--repo")
        .arg(&config_repo())
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("couldn't start {action}: {error}"))
}

fn config_repo() -> String {
    load().repo
}

pub fn restic_init() -> Result<(), String> {
    let config = load();
    if !valid_repo(&config.repo) || !config.has_password {
        return Err("set a repository and password first".into());
    }
    run_restic("restic init", &["init".into()], &config.password)
}

pub fn restic_backup() -> Result<(), String> {
    let config = load();
    if !valid_repo(&config.repo) || !config.has_password {
        return Err("set a repository and password first".into());
    }
    run_restic(
        "restic backup",
        &["backup".into(), home().to_string_lossy().into_owned()],
        &config.password,
    )
}

pub fn restic_restore(snapshot: &str, target: &str, dry_run: bool) -> Result<(), String> {
    if !valid_restore_target(target) {
        return Err("choose an absolute folder outside the home directory".into());
    }
    let config = load();
    if !valid_repo(&config.repo) || !config.has_password {
        return Err("set a repository and password first".into());
    }
    let mut args = vec!["restore".to_string(), snapshot.to_string(), "--target".into(), target.into()];
    if dry_run {
        args.push("--dry-run".into());
    }
    run_restic("restic restore", &args, &config.password)
}

pub fn default_restore_dir(snapshot: &str) -> String {
    home().join(format!("bos-restore-{snapshot}")).to_string_lossy().into_owned()
}

pub fn schedule() -> String {
    let enabled = Command::new("systemctl")
        .args(["--user", "is-enabled", "bos-settings-backup.timer"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if !enabled {
        return "off".into();
    }
    let text = std::fs::read_to_string(
        config_dir().join("systemd/user/bos-settings-backup.timer"),
    )
    .unwrap_or_default();
    if text.contains("OnCalendar=weekly") {
        "weekly".into()
    } else {
        "daily".into()
    }
}

pub fn set_schedule(value: &str) -> Result<(), String> {
    if value == "off" {
        let out = Command::new("systemctl")
            .args(["--user", "disable", "--now", "bos-settings-backup.timer"])
            .output()
            .map_err(|error| error.to_string())?;
        return if out.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        };
    }
    if !["daily", "weekly"].contains(&value) {
        return Err("unknown schedule".into());
    }
    let config = load();
    if !valid_repo(&config.repo) || !config.has_password {
        return Err("set a repository and password first".into());
    }
    let dir = config_dir().join("systemd/user");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let repo = config.repo.replace('\\', "\\\\").replace('"', "\\\"");
    let pass = dir_path().join("restic.pass");
    let service = format!(
        "[Unit]\nDescription=BOS personal file backup\n[Service]\nType=oneshot\nNice=10\nEnvironment=RESTIC_PASSWORD_FILE={}\nExecStart=/usr/bin/restic --repo \"{}\" backup {}\n",
        pass.display(),
        repo,
        home().display()
    );
    let timer = format!(
        "[Unit]\nDescription=Scheduled BOS personal file backup\n[Timer]\nOnCalendar={value}\nPersistent=true\nRandomizedDelaySec=15m\n[Install]\nWantedBy=timers.target\n"
    );
    std::fs::write(dir.join("bos-settings-backup.service"), service).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("bos-settings-backup.timer"), timer).map_err(|error| error.to_string())?;
    for args in [
        vec!["--user", "daemon-reload"],
        vec!["--user", "enable", "--now", "bos-settings-backup.timer"],
    ] {
        let out = Command::new("systemctl")
            .args(args)
            .output()
            .map_err(|error| error.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
    }
    Ok(())
}

pub fn snapshot_available() -> (bool, bool) {
    let config = Path::new("/etc/snapper/configs/root").is_file();
    let live = Path::new("/run/archiso").exists();
    (config, live)
}

pub fn snapshots() -> Result<Vec<Snapshot>, String> {
    let out = Command::new("snapper")
        .args(["--jsonout", "list"])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let Ok(root) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return Err("snapper returned unreadable output".into());
    };
    Ok(root
        .get("root")
        .and_then(|rows| rows.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let number = row.get("number")?.as_u64()?.to_string();
                    Some(Snapshot {
                        number,
                        date: row.get("date").and_then(|value| value.as_str()).unwrap_or("").to_string(),
                        description: row
                            .get("description")
                            .and_then(|value| value.as_str())
                            .unwrap_or("Snapshot")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

pub fn create_snapshot(description: &str) -> Result<Option<String>, String> {
    if !Path::new("/etc/snapper/configs/root").is_file() {
        return Ok(None);
    }
    let out = Command::new("pkexec")
        .args([
            "snapper",
            "-c",
            "root",
            "create",
            "--type",
            "single",
            "--description",
            description,
            "--cleanup-algorithm",
            "number",
            "--print-number",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !out.status.success() {
        return Err(crate::util::fail_output(&out, "Creating a safety snapshot"));
    }
    let number = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !crate::util::valid_number_id(&number) {
        return Err("Snapper did not return a snapshot number".into());
    }
    Ok(Some(number))
}

pub fn create_update_snapshot() -> Result<Option<String>, String> {
    create_snapshot("Before BOS Settings update")
}

pub fn create_manual_snapshot() -> Result<String, String> {
    create_snapshot("Manual BOS Settings snapshot")
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No root snapshot configuration exists".to_string())
}

pub fn restore_snapshot(number: &str) -> Result<(), String> {
    if !crate::util::valid_number_id(number) || number.starts_with('0') {
        return Err("invalid snapshot number".into());
    }
    let confirmation = format!("RESTORE {number}\n");
    if crate::util::run_with_stdin(
        &["pkexec", "bos-restore-snapshot", number],
        &confirmation,
    ) {
        Ok(())
    } else {
        Err("The snapshot could not be restored. The password prompt may have been cancelled.".into())
    }
}

pub fn delete_snapshot(number: &str) -> Result<(), String> {
    if !number.chars().all(|ch| ch.is_ascii_digit()) || number.is_empty() {
        return Err("invalid snapshot number".into());
    }
    let out = Command::new("snapper")
        .args(["delete", number])
        .output()
        .map_err(|error| error.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn reboot() {
    let _ = Command::new("systemctl").arg("reboot").spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_validation_is_strict() {
        assert!(valid_repo("/mnt/backup"));
        assert!(valid_repo("sftp:me@host:/backup"));
        assert!(!valid_repo("relative/path"));
        assert!(!valid_repo("sftp:host:/backup"));
    }

    #[test]
    fn restore_targets_must_be_absolute() {
        assert!(valid_restore_target("/home/me/restored"));
        assert!(!valid_restore_target("restored"));
    }
}
