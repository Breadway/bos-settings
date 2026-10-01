use serde::Serialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::util;

#[derive(Serialize, Clone)]
pub struct SnapshotRow {
    number: String,
    date: String,
    description: String,
}

/// `Err` carries snapper's trimmed stderr; distinct from `Ok(vec![])`
/// (snapper works fine, there just aren't any snapshots yet).
#[tauri::command]
pub async fn get_snapshots() -> Result<Vec<SnapshotRow>, String> {
    // NOTE: the real flag is --columns, not --output-cols (snapper rejects
    // that outright); confirmed against snapper 0.13's own --help.
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
            Some(SnapshotRow {
                number,
                date: cols.next()?.trim().to_string(),
                description: cols.next()?.trim().to_string(),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn delete_snapshot(number: String) -> Result<(), String> {
    if !util::valid_number_id(&number) {
        return Err("invalid snapshot number".into());
    }
    let output = Command::new("snapper")
        .args(["delete", &number])
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if output.success() {
        Ok(())
    } else {
        Err("snapper delete exited with an error: the snapshot wasn't removed.".into())
    }
}

#[tauri::command]
pub async fn restore_snapshot(number: String) -> Result<(), String> {
    if !util::valid_number_id(&number) || number.starts_with('0') {
        return Err("invalid snapshot number".into());
    }
    restore_snapshot_with("pkexec", "bos-restore-snapshot", &number).await
}

async fn restore_snapshot_with(launcher: &str, helper: &str, number: &str) -> Result<(), String> {
    let mut child = Command::new(launcher)
        .args([helper, number])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("couldn't start snapshot restore: {e}"))?;

    // The web view has already asked for explicit confirmation. Answer the
    // helper's terminal confirmation through pkexec's stdin so it can run
    // without an attached terminal.
    let confirmation = format!("RESTORE {number}\n");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(confirmation.as_bytes())
            .await
            .map_err(|e| format!("couldn't confirm snapshot restore: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("couldn't wait for snapshot restore: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!(
                "snapshot restore failed with status {}",
                output.status
            ))
        } else {
            Err(stderr)
        }
    }
}

/// BOS boots with root pinned to a named subvolume (grub emits
/// rootflags=subvol=@), so `snapper rollback`'s usual mechanism has no
/// effect here. The real way back is grub-btrfs, which generates a GRUB
/// submenu entry per snapshot; this just reboots so the user can pick it.
#[tauri::command]
pub fn reboot_system() {
    let _ = std::process::Command::new("systemctl")
        .arg("reboot")
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::{restore_snapshot, restore_snapshot_with};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    #[tokio::test]
    async fn restore_rejects_zero_without_launching_a_command() {
        assert_eq!(
            restore_snapshot("0".into()).await.unwrap_err(),
            "invalid snapshot number"
        );
    }

    #[tokio::test]
    async fn restore_passes_the_snapshot_and_explicit_confirmation_to_helper() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-snapshot-restore-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        let args_file = dir.join("args");
        let input_file = dir.join("confirmation");
        let helper = dir.join("bos-restore-snapshot");
        let launcher = dir.join("pkexec");
        fs::write(
            &helper,
            format!(
                "#!/bin/sh\nIFS= read -r reply\nprintf '%s\\n' \"$reply\" > '{}'\n[ \"$reply\" = \"RESTORE $1\" ]\n",
                input_file.display()
            ),
        )
        .unwrap();
        fs::write(
            &launcher,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexec \"$@\"\n",
                args_file.display()
            ),
        )
        .unwrap();
        for path in [&helper, &launcher] {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(path, permissions).unwrap();
        }

        restore_snapshot_with(launcher.to_str().unwrap(), helper.to_str().unwrap(), "42")
            .await
            .unwrap();

        assert_eq!(
            fs::read_to_string(&args_file).unwrap(),
            format!("{}\n42\n", helper.display())
        );
        assert_eq!(fs::read_to_string(&input_file).unwrap(), "RESTORE 42\n");
        fs::remove_dir_all(dir).unwrap();
    }
}
