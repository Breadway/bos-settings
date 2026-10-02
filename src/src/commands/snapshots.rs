use serde::Serialize;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::util;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SnapshotRow {
    number: String,
    date: String,
    description: String,
    /// `single`, `pre` or `post` (snapper's snapshot type).
    kind: String,
    /// For a `post` snapshot, the number of the `pre` snapshot it closes.
    pre_number: Option<String>,
    /// snapper's cleanup algorithm (`number`, `timeline`, `important`, or empty).
    cleanup: String,
}

/// Parse `snapper --jsonout list`. The machine output is keyed by config
/// name (`{"root": [ {..}, .. ]}`); BOS ships one config (`root`), so prefer
/// that key and fall back to the first config present. snapper's human table
/// separates columns with U+2502 (box drawing), not ASCII `|`, so the table
/// is deliberately never parsed.
///
/// Snapshot 0 ("current") always exists, can't be rolled back to or deleted,
/// and isn't a real snapshot, so it is dropped.
fn parse_snapper_json(text: &str) -> Result<Vec<SnapshotRow>, String> {
    let root: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("couldn't read snapper output: {e}"))?;
    let configs = root
        .as_object()
        .ok_or_else(|| "unexpected snapper output".to_string())?;
    let list = configs
        .get("root")
        .or_else(|| configs.values().next())
        .and_then(|v| v.as_array())
        .ok_or_else(|| "snapper reported no configs".to_string())?;
    Ok(list
        .iter()
        .filter_map(|snap| {
            let number = snap.get("number")?.as_u64()?;
            if number == 0 {
                return None;
            }
            let text = |key: &str| {
                snap.get(key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string()
            };
            Some(SnapshotRow {
                number: number.to_string(),
                date: text("date"),
                description: text("description"),
                kind: text("type"),
                pre_number: snap
                    .get("pre-number")
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string()),
                cleanup: text("cleanup"),
            })
        })
        .collect())
}

/// `Err` carries snapper's trimmed stderr; distinct from `Ok(vec![])`
/// (snapper works fine, there just aren't any snapshots yet).
#[tauri::command]
pub async fn get_snapshots() -> Result<Vec<SnapshotRow>, String> {
    let output = Command::new("snapper")
        .args(["--jsonout", "list"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        eprintln!("bos-settings: snapper list failed: {stderr}");
        return Err(stderr);
    }
    parse_snapper_json(&String::from_utf8_lossy(&output.stdout))
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
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Verbatim `snapper --jsonout list` (snapper 0.13.1) from a BOS machine.
    const REAL_JSON: &str = r#"{
  "root": [
    { "subvolume": "/", "number": 0, "default": false, "active": false, "type": "single",
      "pre-number": null, "date": "", "user": "root", "used-space": null, "cleanup": "",
      "description": "current", "userdata": null },
    { "subvolume": "/", "number": 1, "default": false, "active": false, "type": "single",
      "pre-number": null, "date": "2026-10-01 15:29:11", "user": "root", "used-space": null,
      "cleanup": "important", "description": "pre vanilla-BOS alignment", "userdata": null },
    { "subvolume": "/", "number": 2, "default": false, "active": false, "type": "pre",
      "pre-number": null, "date": "2026-10-01 15:30:19", "user": "root", "used-space": null,
      "cleanup": "number", "description": "pacman -S --needed --noconfirm gnome-disk-utility gufw mission-center re",
      "userdata": null },
    { "subvolume": "/", "number": 3, "default": false, "active": false, "type": "post",
      "pre-number": 2, "date": "2026-10-01 15:31:06", "user": "root", "used-space": null,
      "cleanup": "number", "description": "gnome-disk-utility gufw mission-center restic sbctl",
      "userdata": null }
  ]
}"#;

    #[test]
    fn parses_real_snapper_json_and_drops_current() {
        let rows = parse_snapper_json(REAL_JSON).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].number, "1");
        assert_eq!(rows[0].date, "2026-10-01 15:29:11");
        assert_eq!(rows[0].description, "pre vanilla-BOS alignment");
        assert_eq!(rows[0].kind, "single");
        assert_eq!(rows[0].cleanup, "important");
        assert_eq!(rows[1].kind, "pre");
        assert_eq!(rows[1].pre_number, None);
        assert_eq!(rows[2].kind, "post");
        assert_eq!(rows[2].pre_number.as_deref(), Some("2"));
    }

    #[test]
    fn empty_config_is_ok_and_empty() {
        let rows = parse_snapper_json(r#"{"root": []}"#).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn falls_back_to_first_config_when_root_is_absent() {
        let rows = parse_snapper_json(
            r#"{"home": [{"number": 7, "type": "single", "pre-number": null,
                "date": "2026-01-02 03:04:05", "cleanup": "", "description": "x"}]}"#,
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].number, "7");
    }

    #[test]
    fn human_table_with_box_drawing_is_not_json() {
        // The old parser split this on ASCII '|'; the machine-output path
        // must reject it rather than silently produce an empty list.
        let table = "# \u{2502} Date \u{2502} Description\n0 \u{2502}  \u{2502} current\n";
        assert!(parse_snapper_json(table).is_err());
    }

    #[test]
    fn garbage_is_an_error_not_an_empty_list() {
        assert!(parse_snapper_json("").is_err());
        assert!(parse_snapper_json("[]").is_err());
        assert!(parse_snapper_json(r#"{"root": 3}"#).is_err());
    }

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
