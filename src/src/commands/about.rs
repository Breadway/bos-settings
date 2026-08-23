//! Read-only system info, plus the one thing worth making writable: hostname.
//! BOS is a rolling release (no fixed version number to show — `os-release`
//! ships `BUILD_ID=rolling` on purpose), so there's no "BOS 1.2.3" readout
//! here the way a point-release distro's About panel would have one.

use serde::Serialize;
use std::fs;
use tokio::process::Command;

#[derive(Serialize)]
pub struct SystemInfo {
    os: String,
    kernel: String,
    cpu: String,
    gpu: String,
    memory: String,
    disk: String,
    uptime: String,
    hostname: String,
}

fn os_pretty_name() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "BOS".to_string())
}

fn hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

async fn kernel() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn cpu() -> String {
    let model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("model name")
                    .map(|v| v.trim_start_matches([':', ' ', '\t']).to_string())
            })
        })
        .unwrap_or_else(|| "unknown".to_string());
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    if cores > 0 {
        format!("{model} ({cores} threads)")
    } else {
        model
    }
}

fn memory() -> String {
    let kb = fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
    });
    match kb {
        Some(kb) => format!("{:.1} GiB", kb as f64 / 1024.0 / 1024.0),
        None => "unknown".to_string(),
    }
}

async fn gpu() -> String {
    let Ok(output) = Command::new("lspci").output().await else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        // "Display controller" covers integrated GPUs some laptop chipsets
        // (this dev laptop's AMD Radeon 860M included) report under instead
        // of "VGA compatible controller" — without it those show "unknown".
        .find(|l| {
            l.contains("VGA compatible controller")
                || l.contains("3D controller")
                || l.contains("Display controller")
        })
        .and_then(|l| l.split(": ").nth(1))
        .unwrap_or("unknown")
        .to_string()
}

async fn disk_usage() -> String {
    let Ok(output) = Command::new("df")
        .args(["-h", "--output=used,size,pcent", "/"])
        .output()
        .await
    else {
        return "unknown".to_string();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            match cols.as_slice() {
                [used, size, pcent] => format!("{used} of {size} used ({pcent})"),
                _ => l.trim().to_string(),
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

async fn uptime() -> String {
    Command::new("uptime")
        .arg("-p")
        .output()
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[tauri::command]
pub async fn get_system_info() -> SystemInfo {
    SystemInfo {
        os: os_pretty_name(),
        kernel: kernel().await,
        cpu: cpu(),
        gpu: gpu().await,
        memory: memory(),
        disk: disk_usage().await,
        uptime: uptime().await,
        hostname: hostname(),
    }
}

/// RFC 1123 labels (digit start allowed), no leading `-`. Linux static
/// hostnames are also capped at `HOST_NAME_MAX` (64).
fn valid_hostname(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.len() > 64 || name.starts_with('-') {
        return false;
    }
    if name.contains('\n') || name.contains('\r') || name.contains('\0') {
        return false;
    }
    name.split('.').all(valid_dns_label)
}

fn valid_dns_label(label: &str) -> bool {
    let b = label.as_bytes();
    if b.is_empty() || b.len() > 63 {
        return false;
    }
    if !b[0].is_ascii_alphanumeric() || !b[b.len() - 1].is_ascii_alphanumeric() {
        return false;
    }
    b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
}

#[tauri::command]
pub async fn set_hostname(name: String) -> Result<(), String> {
    let name = name.trim();
    if !valid_hostname(name) {
        return Err("invalid hostname".into());
    }
    let output = Command::new("pkexec")
        .args(["hostnamectl", "set-hostname", name])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_rfc1123() {
        assert!(valid_hostname("bos"));
        assert!(valid_hostname("bos.local"));
        assert!(valid_hostname("a1-b"));
        assert!(valid_hostname("1host"));
        assert!(!valid_hostname(""));
        assert!(!valid_hostname("-bos"));
        assert!(!valid_hostname("bos-"));
        assert!(!valid_hostname("-foo.bar"));
        assert!(!valid_hostname("foo_bar"));
        assert!(!valid_hostname("bos\n-set-hostname evil"));
        assert!(!valid_hostname("--help"));
        assert!(!valid_hostname(&"a".repeat(65)));
    }
}
