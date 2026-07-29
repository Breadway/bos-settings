//! `--screenshot` CLI mode: capture the settings window via
//! `bread-screenshots`, then exit — driven by `bread-ecosystem`'s
//! `bread-capture` orchestrator, or run standalone for one-off captures.
//!
//! First pass at a Tauri (webview, not raw GTK4) target: there's no
//! `connect_map`/`glib` signal to hook here the way every other bread-
//! ecosystem app's screenshot mode does, since the window is owned by
//! tao/wry, not gtk4-rs directly. Instead this just waits a fixed
//! [`SETTLE_DELAY`] on Tauri's own async runtime after `setup()` runs —
//! longer than the native apps' settle delays, since a webview's first
//! paint means a full page load (JS bundle parse + Svelte mount), not just
//! GTK widget layout. The window itself is a plain, non-layer-shell
//! toplevel (per `tauri.conf.json`'s fixed 960x640 size), so — same
//! reasoning as breadman/breadhelp — a full known-size canvas capture is
//! enough; no geometry to track.
//!
//! Only one view ("default", the settings window's initial landing
//! section) is wired up so far. The frontend is a Svelte SPA with its own
//! sidebar routing for each settings section (Appearance, Network,
//! Bluetooth, ...) — capturing one of those specifically would mean
//! passing a route via a URL/hash on window creation and is real, separate
//! frontend work, deferred past this first pass (which exists to prove the
//! isolated-headless-Sway pipeline works against a Tauri/webview window at
//! all, not to reach full view parity with the native GTK4 apps).

use std::path::PathBuf;
use std::time::Duration;

const SETTLE_DELAY: Duration = Duration::from_millis(2000);
const KNOWN_VIEWS: &[&str] = &["default"];

pub struct ScreenshotRequest {
    pub output: PathBuf,
    pub width: u32,
    pub height: u32,
}

/// `None` for a normal run. Exits the process with an error for an unknown
/// view, or if `--screenshot` was given without `--output` — before any
/// Tauri setup happens.
pub fn parse(args: &[String]) -> Option<ScreenshotRequest> {
    let mut view = None;
    let mut output = None;
    let mut width = 1920u32;
    let mut height = 1080u32;
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--screenshot" => view = it.next().cloned(),
            "--output" => output = it.next().cloned(),
            "--width" => {
                if let Some(v) = it.next().and_then(|s| s.parse().ok()) {
                    width = v;
                }
            }
            "--height" => {
                if let Some(v) = it.next().and_then(|s| s.parse().ok()) {
                    height = v;
                }
            }
            _ => {}
        }
    }
    let view = view?;
    if !KNOWN_VIEWS.contains(&view.as_str()) {
        eprintln!(
            "bos-settings: unknown screenshot view '{view}' (known: {})",
            KNOWN_VIEWS.join(", ")
        );
        std::process::exit(1);
    }
    let Some(output) = output else {
        eprintln!("bos-settings: --screenshot requires --output");
        std::process::exit(1);
    };
    Some(ScreenshotRequest { output: output.into(), width, height })
}

/// Schedule the capture-then-exit sequence. Called once from `setup()`.
pub fn dispatch(req: ScreenshotRequest) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SETTLE_DELAY).await;
        finish(bread_screenshots::capture_region(
            0,
            0,
            req.width as i32,
            req.height as i32,
            &req.output,
        ));
    });
}

fn finish(result: anyhow::Result<()>) {
    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("bos-settings: screenshot capture failed: {e}");
            std::process::exit(1);
        }
    }
}
