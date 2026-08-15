//! Shared event-streaming runner for the genuinely long-running operations
//! (package/firmware updates) where the GTK app treated output as "watch
//! the log scroll" — the Tauri-side analog of `stream_command_then`'s
//! async_channel → glib::spawn_future_local pipeline, using Tauri's event
//! bus instead of a GLib main-loop channel.
//!
//! The runner itself is *not* a Tauri command. A generic argv runner was
//! an arbitrary-command primitive; each public command below hardcodes the
//! program and the allowed argument shape.

use serde::Serialize;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone, Serialize)]
pub(crate) struct CmdOutputEvent {
    session_id: String,
    line: String,
}

pub(crate) fn emit_line(app: &AppHandle, session_id: &str, line: &str) {
    let _ = app.emit(
        "cmd-output",
        CmdOutputEvent {
            session_id: session_id.to_string(),
            line: line.to_string(),
        },
    );
}

/// Runs a hardcoded `program args...`, emitting one `cmd-output` event per
/// line of stdout/stderr (tagged with `session_id` so the frontend can route
/// concurrent streams), and resolves to whether it exited successfully.
pub(crate) async fn run_hardcoded(
    app: AppHandle,
    session_id: String,
    program: &str,
    args: &[&str],
) -> bool {
    run_hardcoded_env(app, session_id, program, args, &[]).await
}

pub(crate) async fn run_hardcoded_env(
    app: AppHandle,
    session_id: String,
    program: &str,
    args: &[&str],
    envs: &[(&str, String)],
) -> bool {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let child = cmd.spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit(
                "cmd-output",
                CmdOutputEvent {
                    session_id,
                    line: format!("Error: {e}"),
                },
            );
            return false;
        }
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let read_stdout = async {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit(
                "cmd-output",
                CmdOutputEvent {
                    session_id: session_id.clone(),
                    line,
                },
            );
        }
    };
    let stderr_app = app.clone();
    let stderr_session = session_id.clone();
    let read_stderr = async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_app.emit(
                "cmd-output",
                CmdOutputEvent {
                    session_id: stderr_session.clone(),
                    line,
                },
            );
        }
    };

    tokio::join!(read_stdout, read_stderr);
    child.wait().await.map(|s| s.success()).unwrap_or(false)
}

/// bakery package names are `foo`, `foo-bar`, `foo_bar` — reject flags,
/// paths, and anything else that would change `bakery update`'s shape.
fn valid_bakery_pkg(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
}

#[tauri::command]
pub async fn bakery_update(app: AppHandle, session_id: String, name: String) -> bool {
    if !valid_bakery_pkg(&name) {
        let _ = app.emit(
            "cmd-output",
            CmdOutputEvent {
                session_id,
                line: format!("Error: invalid bakery package name '{name}'"),
            },
        );
        return false;
    }
    run_hardcoded(app, session_id, "bakery", &["update", &name]).await
}

#[tauri::command]
pub async fn bakery_list(app: AppHandle, session_id: String) -> bool {
    run_hardcoded(app, session_id, "bakery", &["list"]).await
}

#[tauri::command]
pub async fn bakery_update_all(app: AppHandle, session_id: String) -> bool {
    run_hardcoded(app, session_id, "bakery", &["update", "--all"]).await
}

#[tauri::command]
pub async fn pacman_system_update(app: AppHandle, session_id: String) -> bool {
    run_hardcoded(
        app,
        session_id,
        "pkexec",
        &["pacman", "-Syu", "--noconfirm"],
    )
    .await
}

#[tauri::command]
pub async fn fwupd_refresh(app: AppHandle, session_id: String) -> bool {
    run_hardcoded(app, session_id, "fwupdmgr", &["refresh"]).await
}

#[tauri::command]
pub async fn fwupd_update(app: AppHandle, session_id: String) -> bool {
    run_hardcoded(app, session_id, "fwupdmgr", &["update", "-y"]).await
}

#[tauri::command]
pub async fn bakery_install(app: AppHandle, session_id: String, name: String) -> bool {
    if let Err(e) = super::util::allowed_bakery_install(&name) {
        emit_line(&app, &session_id, &format!("Error: {e}"));
        return false;
    }
    run_hardcoded(app, session_id, "bakery", &["-y", "install", &name]).await
}

#[tauri::command]
pub async fn pacman_install(app: AppHandle, session_id: String, packages: Vec<String>) -> bool {
    let names = match super::util::allowed_pacman_packages(&packages) {
        Ok(n) => n,
        Err(e) => {
            emit_line(&app, &session_id, &format!("Error: {e}"));
            return false;
        }
    };
    let mut args: Vec<String> = vec![
        "pacman".into(),
        "-S".into(),
        "--noconfirm".into(),
        "--".into(),
    ];
    args.extend(names);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_hardcoded(app, session_id, "pkexec", &refs).await
}

#[cfg(test)]
mod tests {
    use super::valid_bakery_pkg;

    #[test]
    fn bakery_pkg_accepts_real_names() {
        assert!(valid_bakery_pkg("breadbar"));
        assert!(valid_bakery_pkg("bos-settings"));
        assert!(valid_bakery_pkg("bread_theme"));
    }

    #[test]
    fn bakery_pkg_rejects_flags_and_paths() {
        assert!(!valid_bakery_pkg(""));
        assert!(!valid_bakery_pkg("--all"));
        assert!(!valid_bakery_pkg("-S"));
        assert!(!valid_bakery_pkg("../evil"));
        assert!(!valid_bakery_pkg("foo bar"));
        assert!(!valid_bakery_pkg("foo;rm"));
    }
}
