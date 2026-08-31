use serde::Serialize;
use tokio::process::Command;

use super::util;

#[derive(Serialize, Clone)]
pub struct SnapshotRow {
    number: String,
    date: String,
    description: String,
}

/// `Err` carries snapper's trimmed stderr — distinct from `Ok(vec![])`
/// (snapper works fine, there just aren't any snapshots yet).
#[tauri::command]
pub async fn get_snapshots() -> Result<Vec<SnapshotRow>, String> {
    // NOTE: the real flag is --columns, not --output-cols (snapper rejects
    // that outright) — confirmed against snapper 0.13's own --help.
    let output = Command::new("snapper")
        .args(["list", "--columns", "number,date,description"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        eprintln!("bos-settings: snapper list failed: {stderr}");
        return Err(stderr);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text
        .lines()
        .skip(2) // header + separator
        .filter_map(|line| {
            let mut cols = line.splitn(3, '|');
            let number = cols.next()?.trim().to_string();
            // Snapshot 0 ("current") always exists, can't be rolled back to
            // or deleted, and isn't a real snapshot.
            if number == "0" {
                return None;
            }
            Some(SnapshotRow { number, date: cols.next()?.trim().to_string(), description: cols.next()?.trim().to_string() })
        })
        .collect())
}

#[tauri::command]
pub async fn delete_snapshot(number: String) -> Result<(), String> {
    if !util::valid_number_id(&number) {
        return Err("invalid snapshot number".into());
    }
    let output = Command::new("snapper").args(["delete", &number]).status().await.map_err(|e| e.to_string())?;
    if output.success() {
        Ok(())
    } else {
        Err("snapper delete exited with an error — the snapshot wasn't removed.".into())
    }
}

/// BOS boots with root pinned to a named subvolume (grub emits
/// rootflags=subvol=@), so `snapper rollback`'s usual mechanism has no
/// effect here. The real way back is grub-btrfs, which generates a GRUB
/// submenu entry per snapshot — this just reboots so the user can pick it.
#[tauri::command]
pub fn reboot_system() {
    let _ = std::process::Command::new("systemctl").arg("reboot").spawn();
}
