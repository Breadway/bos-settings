//! restic backups of `$HOME`. Repo path + password live in
//! `~/.config/bos-settings/backup.toml` (0600). The password is write-only
//! to the webview — empty on save keeps the stored secret.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tokio::process::Command;

use super::config;
use super::streaming;
use super::util::{self, command_exists, fail_output};

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/root"))
}

fn backup_toml() -> PathBuf {
    util::bos_settings_dir().join("backup.toml")
}

#[derive(Clone)]
pub struct BackupSecrets {
    pub repo: String,
    pub password: Option<String>,
}

impl BackupSecrets {
    fn empty() -> Self {
        Self {
            repo: String::new(),
            password: None,
        }
    }
}

pub fn load_secrets() -> BackupSecrets {
    load_secrets_from(&backup_toml())
}

fn load_secrets_from(path: &Path) -> BackupSecrets {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BackupSecrets::empty();
    };
    let doc = text.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    BackupSecrets {
        repo: config::get_str(&doc, &["repo"]).unwrap_or_default(),
        password: config::get_str(&doc, &["password"]).filter(|s| !s.is_empty()),
    }
}

fn save_secrets_to(path: &Path, repo: &str, password: Option<&str>) -> Result<(), String> {
    let existing = load_secrets_from(path);
    let password = match password.map(str::trim).filter(|s| !s.is_empty()) {
        Some(p) => Some(p.to_string()),
        None => existing.password,
    };
    let mut doc = toml_edit::DocumentMut::new();
    config::set_str(&mut doc, &["repo"], repo.trim());
    if let Some(p) = password.as_deref() {
        config::set_str(&mut doc, &["password"], p);
    }
    util::write_secure(path, &doc.to_string())
}

#[derive(Serialize)]
pub struct BackupStatus {
    restic_installed: bool,
    repo: String,
    has_password: bool,
    snapshots: Vec<ResticSnapshot>,
    error: Option<String>,
    home: String,
}

#[derive(Serialize, Clone)]
pub struct ResticSnapshot {
    id: String,
    time: String,
    paths: Vec<String>,
}

#[tauri::command]
pub fn get_backup_config() -> BackupStatus {
    let s = load_secrets();
    BackupStatus {
        restic_installed: command_exists("restic"),
        repo: s.repo,
        has_password: s.password.is_some(),
        snapshots: Vec::new(),
        error: None,
        home: home_dir().to_string_lossy().into_owned(),
    }
}

#[derive(Deserialize)]
pub struct SaveBackupInput {
    repo: String,
    #[serde(default)]
    password: Option<String>,
}

#[tauri::command]
pub fn save_backup_config(input: SaveBackupInput) -> Result<(), String> {
    if !valid_repo(&input.repo) {
        return Err("repo must be an absolute path or sftp:user@host:path".into());
    }
    save_secrets_to(&backup_toml(), &input.repo, input.password.as_deref())
}

pub fn valid_repo(repo: &str) -> bool {
    let repo = repo.trim();
    if repo.is_empty() || repo.len() > 512 || repo.contains('\n') || repo.contains('\0') {
        return false;
    }
    if let Some(rest) = repo.strip_prefix("sftp:") {
        return !rest.is_empty() && rest.contains('@') && rest.contains(':') && !rest.contains(' ');
    }
    std::path::Path::new(repo).is_absolute()
}

fn require_ready() -> Result<BackupSecrets, String> {
    if !command_exists("restic") {
        return Err("restic is not installed".into());
    }
    let s = load_secrets();
    if !valid_repo(&s.repo) {
        return Err("set a repository path first".into());
    }
    if s.password.is_none() {
        return Err("set a repository password first".into());
    }
    Ok(s)
}

fn restic_args<'a>(repo: &'a str, extra: &'a [&'a str]) -> Vec<&'a str> {
    let mut args = vec!["--repo", repo];
    args.extend_from_slice(extra);
    args
}

#[tauri::command]
pub async fn restic_init(app: AppHandle, session_id: String) -> bool {
    let Ok(s) = require_ready() else {
        streaming::emit_line(
            &app,
            &session_id,
            "Error: configure repo and password first",
        );
        return false;
    };
    let password = s.password.clone().unwrap_or_default();
    let extra = ["init"];
    let args = restic_args(&s.repo, &extra);
    streaming::run_hardcoded_env(
        app,
        session_id,
        "restic",
        &args,
        &[("RESTIC_PASSWORD", password)],
    )
    .await
}

fn exclude_args(home: &str) -> Vec<String> {
    let extras = [
        ".cache",
        ".local/share/Trash",
        ".local/share/Steam",
        ".local/share/containers",
        ".npm",
        ".cargo/registry",
        ".cargo/git",
        ".rustup",
        ".var/app",
    ];
    let mut args = vec![
        "--exclude-caches".into(),
        "--exclude".into(),
        "node_modules".into(),
        "--exclude".into(),
        "target".into(),
        "--exclude".into(),
        ".git".into(),
    ];
    for rel in extras {
        args.push("--exclude".into());
        args.push(format!("{home}/{rel}"));
    }
    args
}

#[tauri::command]
pub async fn restic_backup(app: AppHandle, session_id: String) -> bool {
    let s = match require_ready() {
        Ok(s) => s,
        Err(e) => {
            streaming::emit_line(&app, &session_id, &format!("Error: {e}"));
            return false;
        }
    };
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let password = s.password.clone().unwrap_or_default();
    let excludes = exclude_args(&home);
    let mut args = vec!["--repo".to_string(), s.repo.clone()];
    args.extend(excludes);
    args.push("backup".into());
    args.push(home);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    streaming::run_hardcoded_env(
        app,
        session_id,
        "restic",
        &arg_refs,
        &[("RESTIC_PASSWORD", password)],
    )
    .await
}

/// `~/bos-restore-<id>`. Never `$HOME` itself — restore writes into a new
/// directory so a bad snapshot cannot clobber the live home.
pub fn default_restore_dir(snapshot: &str) -> PathBuf {
    home_dir().join(format!("bos-restore-{snapshot}"))
}

fn normalize_abs(path: &Path) -> PathBuf {
    path.components().collect()
}

/// Absolute path, not `$HOME` and not `/`. Empty target means the default.
pub fn valid_restore_target(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let s = path.to_string_lossy();
    if s.is_empty() || s.len() > 512 || s.contains('\n') || s.contains('\0') {
        return false;
    }
    let normalized = normalize_abs(path);
    if normalized == *"/" {
        return false;
    }
    normalized != normalize_abs(&home_dir())
}

fn resolve_restore_target(snapshot: &str, target: Option<&str>) -> Result<PathBuf, String> {
    if !valid_snapshot_id(snapshot) {
        return Err("invalid snapshot id".into());
    }
    let dest = match target.map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => PathBuf::from(t),
        None => default_restore_dir(snapshot),
    };
    if !valid_restore_target(&dest) {
        return Err(
            "restore target must be an absolute path that is not $HOME (default is ~/bos-restore-<id>)"
                .into(),
        );
    }
    Ok(dest)
}

async fn run_restic_restore(
    app: AppHandle,
    session_id: String,
    snapshot: String,
    target: Option<String>,
    dry_run: bool,
) -> bool {
    let s = match require_ready() {
        Ok(s) => s,
        Err(e) => {
            streaming::emit_line(&app, &session_id, &format!("Error: {e}"));
            return false;
        }
    };
    let snap = snapshot.trim();
    let dest = match resolve_restore_target(snap, target.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            streaming::emit_line(&app, &session_id, &format!("Error: {e}"));
            return false;
        }
    };
    let dest_s = dest.to_string_lossy().into_owned();
    let password = s.password.clone().unwrap_or_default();
    let mut extra = vec![
        "restore".to_string(),
        snap.to_string(),
        "--target".into(),
        dest_s.clone(),
    ];
    if dry_run {
        extra.push("--dry-run".into());
    }
    streaming::emit_line(
        &app,
        &session_id,
        &format!(
            "{} {snap} → {dest_s}",
            if dry_run {
                "Dry-run restore"
            } else {
                "Restoring"
            }
        ),
    );
    let extra_refs: Vec<&str> = extra.iter().map(String::as_str).collect();
    let args = restic_args(&s.repo, &extra_refs);
    streaming::run_hardcoded_env(
        app,
        session_id,
        "restic",
        &args,
        &[("RESTIC_PASSWORD", password)],
    )
    .await
}

#[tauri::command]
pub async fn restic_restore_dry_run(
    app: AppHandle,
    session_id: String,
    snapshot: String,
    target: Option<String>,
) -> bool {
    run_restic_restore(app, session_id, snapshot, target, true).await
}

#[tauri::command]
pub async fn restic_restore(
    app: AppHandle,
    session_id: String,
    snapshot: String,
    target: Option<String>,
) -> bool {
    run_restic_restore(app, session_id, snapshot, target, false).await
}

fn valid_snapshot_id(id: &str) -> bool {
    if id == "latest" {
        return true;
    }
    let bytes = id.as_bytes();
    !bytes.is_empty() && bytes.len() <= 64 && bytes.iter().all(|b| b.is_ascii_hexdigit())
}

#[tauri::command]
pub async fn list_restic_snapshots() -> Result<Vec<ResticSnapshot>, String> {
    let s = require_ready()?;
    let password = s.password.clone().unwrap_or_default();
    let output = Command::new("restic")
        .args(["--repo", &s.repo, "snapshots", "--json"])
        .env("RESTIC_PASSWORD", password)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(fail_output(&output, "restic snapshots"));
    }
    parse_snapshots(&output.stdout)
}

fn parse_snapshots(bytes: &[u8]) -> Result<Vec<ResticSnapshot>, String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("restic json: {e}"))?;
    let Some(arr) = v.as_array() else {
        return Ok(Vec::new());
    };
    Ok(arr
        .iter()
        .filter_map(|s| {
            let id = s
                .get("short_id")
                .or_else(|| s.get("id"))
                .and_then(|x| x.as_str())?
                .to_string();
            let time = s
                .get("time")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let paths = s
                .get("paths")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|p| p.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            Some(ResticSnapshot { id, time, paths })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_accepts_abs_and_sftp() {
        assert!(valid_repo("/mnt/backup/bos"));
        assert!(valid_repo("sftp:user@host:/backups/bos"));
        assert!(!valid_repo("relative/path"));
        assert!(!valid_repo("sftp:nocolon"));
        assert!(!valid_repo("sftp:user host:/x"));
        assert!(!valid_repo(""));
    }

    #[test]
    fn snapshot_id_hex_or_latest() {
        assert!(valid_snapshot_id("latest"));
        assert!(valid_snapshot_id("a1b2c3d4"));
        assert!(!valid_snapshot_id("../x"));
        assert!(!valid_snapshot_id("latest;rm"));
    }

    #[test]
    fn write_secure_is_0600_and_keeps_password() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("backup.toml");
        save_secrets_to(&path, "/tmp/repo", Some("hunter2")).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("hunter2"));
        assert!(text.contains("/tmp/repo"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "backup.toml must be 0600, got {mode:o}");
        }
        save_secrets_to(&path, "/tmp/repo2", Some("")).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("hunter2"), "empty password keeps secret");
        assert!(text.contains("/tmp/repo2"));
        let loaded = load_secrets_from(&path);
        assert_eq!(loaded.password.as_deref(), Some("hunter2"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_restic_json() {
        let json = br#"[{"short_id":"abc123","time":"2026-08-15T01:00:00Z","paths":["/home/a"]}]"#;
        let v = parse_snapshots(json).unwrap();
        assert_eq!(v[0].id, "abc123");
        assert_eq!(v[0].paths[0], "/home/a");
    }

    #[test]
    fn restore_defaults_to_bos_restore_id_not_home() {
        let dest = default_restore_dir("a1b2c3d4");
        let home = home_dir();
        assert_eq!(dest, home.join("bos-restore-a1b2c3d4"));
        assert_ne!(dest, home);
        assert!(valid_restore_target(&dest));
        assert!(!valid_restore_target(&home));
        assert!(!valid_restore_target(Path::new("/")));
        assert!(!valid_restore_target(Path::new("relative/path")));
        assert!(valid_restore_target(Path::new("/tmp/bos-restore-custom")));
        let resolved = resolve_restore_target("latest", None).unwrap();
        assert_eq!(resolved, home.join("bos-restore-latest"));
        assert!(resolve_restore_target("latest", Some(home.to_str().unwrap())).is_err());
    }

    #[test]
    fn exclude_covers_caches_and_containers() {
        let args = exclude_args("/home/a");
        let joined = args.join(" ");
        assert!(joined.contains("/home/a/.cache"));
        assert!(joined.contains("/home/a/.local/share/Trash"));
        assert!(joined.contains("/home/a/.local/share/Steam"));
        assert!(joined.contains("/home/a/.local/share/containers"));
        assert!(joined.contains("node_modules"));
        assert!(joined.contains("target"));
        assert!(joined.contains(".git"));
    }
}
