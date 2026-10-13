//! Typed storage operations. Paths come from XDG and fixed system locations.
use super::util;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Serialize)]
pub struct DiskUsage {
    path: String,
    /// Backing device; `/` and `/home` on one Btrfs volume share it and are
    /// shown once instead of as two identical bars.
    #[serde(skip)]
    source: String,
    total: u64,
    used: u64,
    available: u64,
}
#[derive(Serialize)]
pub struct CacheEntry {
    name: String,
    bytes: u64,
}
#[derive(Serialize)]
pub struct StorageInfo {
    disks: Vec<DiskUsage>,
    package_cache: Option<u64>,
    app_cache: Option<u64>,
    trash: Option<u64>,
    caches: Vec<CacheEntry>,
    errors: Vec<String>,
}
fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Home directory is unavailable".into())
}
fn cache_home() -> Result<PathBuf, String> {
    Ok(std::env::var_os("XDG_CACHE_HOME")
        .filter(|p| Path::new(p).is_absolute())
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".cache")))
}
fn data_home() -> Result<PathBuf, String> {
    Ok(std::env::var_os("XDG_DATA_HOME")
        .filter(|p| Path::new(p).is_absolute())
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".local/share")))
}
fn parse_df(text: &str, path: &Path) -> Option<DiskUsage> {
    let fields: Vec<_> = text.lines().nth(1)?.split_whitespace().collect();
    Some(DiskUsage {
        path: path.display().to_string(),
        source: fields.first()?.to_string(),
        total: fields.get(1)?.parse().ok()?,
        used: fields.get(2)?.parse().ok()?,
        available: fields.get(3)?.parse().ok()?,
    })
}
async fn disk(path: &Path) -> Result<DiskUsage, String> {
    let out = Command::new("df")
        .args(["-B1", "--output=source,size,used,avail", "--"])
        .arg(path)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(util::fail_output(&out, "Reading disk usage"));
    }
    parse_df(&String::from_utf8_lossy(&out.stdout), path)
        .ok_or_else(|| "Could not read disk usage".into())
}
async fn size(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    let out = Command::new("du")
        .args(["-s", "-B1", "--"])
        .arg(path)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    // du exits 1 when any folder inside can't be read (container overlays,
    // root-only parts of /etc and /var) but still prints the total of what it
    // could read. Use that: slightly low beats "Unavailable" and a wall of
    // permission errors.
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| util::fail_output(&out, "Measuring storage"))
}
#[tauri::command]
pub async fn get_storage_info() -> Result<StorageInfo, String> {
    let home = home()?;
    let cache = cache_home()?;
    let trash = data_home()?.join("Trash");
    let (root, home_disk, packages, apps, trash_size) = tokio::join!(
        disk(Path::new("/")),
        disk(&home),
        size(Path::new("/var/cache/pacman/pkg")),
        size(&cache),
        size(&trash)
    );
    let mut errors = Vec::new();
    let mut disks = Vec::new();
    for result in [root, home_disk] {
        match result {
            Ok(v) if disks.iter().any(|d: &DiskUsage| d.source == v.source) => {}
            Ok(v) => disks.push(v),
            Err(e) => errors.push(e),
        }
    }
    let mut measurement = |result: Result<u64, String>| match result {
        Ok(v) => Some(v),
        Err(e) => {
            errors.push(e);
            None
        }
    };
    let package_cache = measurement(packages);
    let app_cache = measurement(apps);
    let trash = measurement(trash_size);
    Ok(StorageInfo {
        disks,
        package_cache,
        app_cache,
        trash,
        caches: Vec::new(),
        errors,
    })
}
#[tauri::command]
pub async fn get_cache_entries() -> Result<Vec<CacheEntry>, String> {
    let cache = cache_home()?;
    if !cache.exists() {
        return Ok(Vec::new());
    }
    let out = Command::new("du")
        .args(["-B1", "--max-depth=1", "--"])
        .arg(&cache)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    // As in size(): unreadable folders make du exit 1 but the rest is fine.
    if out.stdout.is_empty() && !out.status.success() {
        return Err(util::fail_output(&out, "Reading app caches"));
    }
    let mut entries = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some((bytes, path)) = line.split_once('\t') else {
            continue;
        };
        let path = Path::new(path);
        if path.parent() != Some(cache.as_path()) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if let Ok(bytes) = bytes.parse() {
            entries.push(CacheEntry {
                name: name.into(),
                bytes,
            });
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.bytes));
    Ok(entries)
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Cleanup {
    PackageCache,
    Trash,
    AppCache { name: String },
}
fn valid_cache_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.len() <= 255
        && !name.contains(['/', '\\', '\0', '\n', '\t'])
}
#[tauri::command]
pub async fn clean_storage(target: Cleanup) -> Result<(), String> {
    let out = match target {
        Cleanup::PackageCache => {
            Command::new("pkexec")
                .args(["paccache", "-r", "-k", "3"])
                .output()
                .await
        }
        Cleanup::Trash => {
            Command::new("gio")
                .args(["trash", "--empty"])
                .output()
                .await
        }
        Cleanup::AppCache { name } => {
            if !valid_cache_name(&name) {
                return Err("Invalid cache name".into());
            }
            let path = cache_home()?.join(&name);
            // Refuse a linked cache directory: never delete a target outside the cache.
            let meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if meta.file_type().is_symlink() || !meta.is_dir() {
                return Err("Only app cache directories can be cleaned".into());
            }
            return tokio::task::spawn_blocking(move || {
                std::fs::remove_dir_all(path).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())?;
        }
    }
    .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(util::fail_output(&out, "Cleaning storage"))
    }
}
#[tauri::command]
pub fn open_storage_folder(cache: bool) -> Result<(), String> {
    let path = if cache { cache_home()? } else { home()? };
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_capacity_and_available_without_confusing_them() {
        let disk = parse_df(
            "Filesystem 1B-blocks Used Avail\n/dev/sda1 1000 600 350\n",
            Path::new("/"),
        )
        .unwrap();
        assert_eq!((disk.total, disk.used, disk.available), (1000, 600, 350));
        assert!(parse_df("header\nbad data\n", Path::new("/")).is_none());
    }
    #[test]
    fn cleaning_cannot_escape_the_cache_directory() {
        for name in [
            "",
            ".",
            "..",
            "../config",
            "/etc",
            "app/../../etc",
            "app\\other",
            "app\nother",
        ] {
            assert!(!valid_cache_name(name), "{name:?}");
        }
        assert!(valid_cache_name("mozilla"));
        assert!(valid_cache_name(".thumbnails"));
    }
}

#[tauri::command]
pub async fn get_unused_dependencies() -> Result<Vec<String>, String> {
    let out = Command::new("pacman")
        .arg("-Qdtq")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !(out.status.success() || out.status.code() == Some(1) && out.stderr.is_empty()) {
        return Err(util::fail_output(&out, "Reviewing unused dependencies"));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|s| util::valid_pkg_name(s))
        .map(String::from)
        .collect())
}
#[tauri::command]
pub async fn remove_unused_dependencies(
    app: tauri::AppHandle,
    session_id: String,
    packages: Vec<String>,
) -> Result<bool, String> {
    let current = get_unused_dependencies().await?;
    if packages.is_empty()
        || packages.len() > 500
        || packages
            .iter()
            .any(|p| !util::valid_pkg_name(p) || !current.contains(p))
    {
        return Err("Review the current unused dependency list before removing packages".into());
    }
    let mut args = vec!["pacman", "-R", "--noconfirm", "--"];
    args.extend(packages.iter().map(String::as_str));
    Ok(super::streaming::run_hardcoded(app, session_id, "pkexec", &args).await)
}

#[derive(Serialize)]
pub struct DirectoryUsage {
    label: String,
    bytes: Option<u64>,
    error: Option<String>,
}
#[tauri::command]
pub async fn get_storage_breakdown() -> Result<Vec<DirectoryUsage>, String> {
    let home = home()?;
    let read = |label: &str, result: Result<u64, String>| match result {
        Ok(bytes) => DirectoryUsage {
            label: label.into(),
            bytes: Some(bytes),
            error: None,
        },
        Err(error) => DirectoryUsage {
            label: label.into(),
            bytes: None,
            error: Some(error),
        },
    };
    let system = async {
        let mut total = 0;
        for directory in ["/usr", "/etc", "/var", "/opt", "/boot"] {
            total += size(Path::new(directory)).await?;
        }
        Ok(total)
    };
    let (system, files, snapshots) =
        tokio::join!(system, size(&home), size(Path::new("/.snapshots")));
    Ok(vec![
        read("System directories", system),
        read("Your files", files),
        read("System snapshots", snapshots),
    ])
}
