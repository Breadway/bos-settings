//! Aggregated Updates page: pacman -Qu, bakery dry-run, fwupd devices.
//! Rollback is Snapshots / grub-btrfs — not `snapper rollback`.

use serde::Serialize;
use tokio::process::Command;

use super::firmware::{get_updatable_firmware, FwDevice};
use super::nvidia::{read_nvidia_offer, NvidiaOffer};
use super::util::strip_ansi;

#[derive(Serialize, Clone)]
pub struct PendingUpdate {
    name: String,
    current: String,
    latest: String,
}

#[derive(Serialize)]
pub struct UpdatesStatus {
    pacman: Vec<PendingUpdate>,
    pacman_error: Option<String>,
    bakery: Vec<PendingUpdate>,
    bakery_error: Option<String>,
    firmware: Vec<FwDevice>,
    nvidia: Option<NvidiaOffer>,
}

#[tauri::command]
pub async fn get_updates_status() -> UpdatesStatus {
    let (pacman, bakery, firmware) = tokio::join!(
        list_pacman_upgrades(),
        list_bakery_outdated(),
        get_updatable_firmware()
    );
    let (pacman, pacman_error) = match pacman {
        Ok(v) => (v, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let (bakery, bakery_error) = match bakery {
        Ok(v) => (v, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    UpdatesStatus {
        pacman,
        pacman_error,
        bakery,
        bakery_error,
        firmware,
        nvidia: read_nvidia_offer(),
    }
}

async fn list_pacman_upgrades() -> Result<Vec<PendingUpdate>, String> {
    let output = Command::new("pacman")
        .args(["-Qu"])
        .output()
        .await
        .map_err(|e| format!("couldn't run pacman: {e}"))?;
    // pacman -Qu exits 1 when there is nothing to upgrade.
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_pacman_qu(&text))
}

fn parse_pacman_qu(text: &str) -> Vec<PendingUpdate> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // "name old -> new" — extra fields after new are ignored.
            let mut parts = line.split_whitespace();
            let name = parts.next()?.to_string();
            let current = parts.next()?.to_string();
            let arrow = parts.next()?;
            if arrow != "->" {
                return None;
            }
            let latest = parts.next()?.to_string();
            Some(PendingUpdate {
                name,
                current,
                latest,
            })
        })
        .collect()
}

async fn list_bakery_outdated() -> Result<Vec<PendingUpdate>, String> {
    let output = Command::new("bakery")
        .args(["--dry-run", "update", "--all"])
        .output()
        .await
        .map_err(|e| format!("couldn't run bakery: {e}"))?;
    let text = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let err = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let combined = format!("{text}\n{err}");
    Ok(parse_bakery_outdated(&combined))
}

/// bakery has no `outdated` subcommand. `--dry-run update --all` is the
/// CLI's own preview of what a track-aware update would change.
fn parse_bakery_outdated(text: &str) -> Vec<PendingUpdate> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(pkg) = parse_would_update(line).or_else(|| parse_updating_arrow(line)) {
            if !out.iter().any(|p: &PendingUpdate| p.name == pkg.name) {
                out.push(pkg);
            }
        }
    }
    out
}

fn parse_would_update(line: &str) -> Option<PendingUpdate> {
    // "dry-run: would update bakery to 0.7.3-dev.…"
    // "Would update  bakery  0.7.3-dev.…"
    let lower = line.to_ascii_lowercase();
    let i = lower.find("would update")?;
    let rest = line[i + "would update".len()..].trim();
    let rest = rest.strip_prefix(':').unwrap_or(rest).trim();
    let rest = rest.strip_prefix("to ").unwrap_or(rest);
    let mut parts = rest.split_whitespace();
    let name = parts.next()?.to_string();
    let mut latest = parts.next().unwrap_or("").to_string();
    if latest.eq_ignore_ascii_case("to") {
        latest = parts.next().unwrap_or("").to_string();
    }
    if name.is_empty() {
        return None;
    }
    Some(PendingUpdate {
        name,
        current: String::new(),
        latest,
    })
}

fn parse_updating_arrow(line: &str) -> Option<PendingUpdate> {
    // "updating bakery 0.7.2 → 0.7.3"
    let line = line.strip_prefix("updating ")?;
    let (name, rest) = line.split_once(' ')?;
    let (current, latest) = rest.split_once('→')?;
    Some(PendingUpdate {
        name: name.trim().to_string(),
        current: current.trim().to_string(),
        latest: latest.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_qu_parses_arrow_lines() {
        let text =
            "linux 6.15.1-1 -> 6.15.2-1\nextra-note\nbos-settings 0.8.0-1 -> 0.8.1-1 [ignored]\n";
        let v = parse_pacman_qu(text);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "linux");
        assert_eq!(v[0].current, "6.15.1-1");
        assert_eq!(v[0].latest, "6.15.2-1");
        assert_eq!(v[1].name, "bos-settings");
    }

    #[test]
    fn bakery_dry_run_parses_would_update_and_arrow() {
        let text = "\
  · breadbar is already at 0.3.2
updating bakery 0.7.2-dev.1 → 0.7.3-dev.2
  dry-run: would update bakery to 0.7.3-dev.2
Would update  breadcast  1.2.3
1 updated, 14 already up to date
";
        let v = parse_bakery_outdated(text);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].name, "bakery");
        assert_eq!(v[0].latest, "0.7.3-dev.2");
        assert_eq!(v[1].name, "breadcast");
        assert_eq!(v[1].latest, "1.2.3");
    }
}
