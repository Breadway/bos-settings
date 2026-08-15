//! `--screenshot` CLI mode: switch the Svelte SPA to the named sidebar
//! section, capture it via grim, then exit — driven by
//! `bread-ecosystem`'s `bread-capture` orchestrator, or run standalone for
//! one-off captures.
//!
//! There's no `connect_map`/`glib` signal to hook here the way every other
//! bread-ecosystem app's screenshot mode does, since the window is owned by
//! tao/wry (Tauri's Linux backend), not gtk4-rs directly. Instead this
//! waits a fixed [`INITIAL_SETTLE_DELAY`] on Tauri's own async runtime after
//! `setup()` runs for the page's first paint (JS bundle parse + Svelte
//! mount) — longer than the native apps' settle delays, since a webview's
//! first paint is a full page load, not just GTK widget layout — then emits
//! a `screenshot-set-view` event the frontend listens for
//! (`+page.svelte`'s `onMount`) to switch `activePage` exactly like a real
//! sidebar click would, then waits [`VIEW_SETTLE_DELAY`] more for that
//! view's own data to load (each section fetches its own state over Tauri
//! commands on mount) before capturing. The window itself is a plain,
//! non-layer-shell toplevel (per `tauri.conf.json`'s fixed 960x640 size),
//! so — same reasoning as breadman/breadhelp — a full known-size canvas
//! capture is enough; no geometry to track.
//!
//! View names match `frontend/src/lib/sidebar.ts`'s item ids exactly (see
//! `KNOWN_VIEWS`) — every one of them has a real registered component (see
//! `frontend/src/lib/views/registry.ts`), no Placeholder fallbacks to skip.

use std::path::PathBuf;
use std::time::Duration;
use tauri::Emitter;

const INITIAL_SETTLE_DELAY: Duration = Duration::from_millis(2000);
/// Applied after switching views — shorter than the initial load (no full
/// page/JS reload, just a component swap + that view's own Tauri-command
/// data fetch), but the About page alone needed 2s for its fetch to land
/// (see the initial-pass commit), so this stays generous rather than
/// re-guessing per view.
const VIEW_SETTLE_DELAY: Duration = Duration::from_millis(2000);

const KNOWN_VIEWS: &[&str] = &[
    "network",
    "breadcrumbs",
    "bluetooth",
    "firewall",
    "sound",
    "power",
    "datetime",
    "hyprland",
    "keybinds",
    "autostart",
    "users",
    "appearance",
    "breadpaper",
    "breadbar",
    "breadbox",
    "breadclip",
    "breadpad",
    "breadsearch",
    "bread",
    "packages",
    "aur",
    "firmware",
    "snapshots",
    "updates",
    "printing",
    "vpn",
    "nightlight",
    "ime",
    "accessibility",
    "defaults",
    "channel",
    "backup",
    "optional",
    "breadlock",
    "breadshot",
    "breadmon",
    "breadhelp",
    "about",
];

pub struct ScreenshotRequest {
    pub view: String,
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
    Some(ScreenshotRequest {
        view,
        output: output.into(),
        width,
        height,
    })
}

/// Schedule the switch-view-then-capture-then-exit sequence. Called once
/// from `setup()`, which is also where `app` (needed to emit the
/// `screenshot-set-view` event) comes from.
pub fn dispatch(req: ScreenshotRequest, app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(INITIAL_SETTLE_DELAY).await;
        if let Err(e) = app.emit("screenshot-set-view", &req.view) {
            eprintln!("bos-settings: failed to emit screenshot-set-view: {e}");
            std::process::exit(1);
        }
        tokio::time::sleep(VIEW_SETTLE_DELAY).await;
        finish(capture_region(
            0,
            0,
            req.width as i32,
            req.height as i32,
            &req.output,
        ));
    });
}

/// Same contract as bread-screenshots::capture_region. That crate is not
/// on bread-ecosystem v0.7.1 (it landed after the tag), so this stays a
/// local grim -g call rather than a branch-pinned git dep.
fn capture_region(x: i32, y: i32, w: i32, h: i32, out: &std::path::Path) -> anyhow::Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let out_str = out
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("output path is not valid UTF-8"))?;
    let geometry = format!("{x},{y} {w}x{h}");
    let result =
        bread_utils::proc::run("grim", &["-g", &geometry, out_str], Duration::from_secs(5));
    if !result.success {
        anyhow::bail!(
            "grim failed for geometry {geometry}: {}",
            result.stderr.trim()
        );
    }
    Ok(())
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
