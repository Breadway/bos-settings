//! Shared event-streaming command runner for the genuinely long-running
//! operations (package/firmware updates) where the GTK app treated output
//! as "watch the log scroll" — the Tauri-side analog of
//! `stream_command_then`'s async_channel → glib::spawn_future_local
//! pipeline, using Tauri's event bus instead of a GLib main-loop channel.
//! Most other commands are simple request/response (see the other modules)
//! since the operations they wrap finish in well under a second.

use serde::Serialize;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone, Serialize)]
struct CmdOutputEvent {
    session_id: String,
    line: String,
}

/// Runs `program args...`, emitting one `cmd-output` event per line of
/// stdout/stderr (tagged with `session_id` so the frontend can route
/// concurrent streams), and resolves to whether it exited successfully —
/// the frontend awaits this call directly rather than needing a second
/// "done" event.
#[tauri::command]
pub async fn run_streaming_command(app: AppHandle, session_id: String, program: String, args: Vec<String>) -> bool {
    let child = Command::new(&program).args(&args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("cmd-output", CmdOutputEvent { session_id, line: format!("Error: {e}") });
            return false;
        }
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let read_stdout = async {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = app.emit("cmd-output", CmdOutputEvent { session_id: session_id.clone(), line });
        }
    };
    let stderr_app = app.clone();
    let stderr_session = session_id.clone();
    let read_stderr = async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = stderr_app.emit("cmd-output", CmdOutputEvent { session_id: stderr_session.clone(), line });
        }
    };

    tokio::join!(read_stdout, read_stderr);
    child.wait().await.map(|s| s.success()).unwrap_or(false)
}
