//! Bridges `bread-theme`'s pywal-derived palette into the webview as CSS
//! custom properties, and keeps it live.
//!
//! `bread-theme` rewrites every file it generates with a temp-then-rename
//! (atomic replace), which kills a direct file watch (inotify reports
//! `DELETE_SELF` and never re-arms) — so this watches the *parent directory*
//! and filters by path instead, the same strategy `bread_theme::gtk`'s
//! `watch_theme_file` / `ensure_themes_watch` use for the GTK apps.
//!
//! Three inputs can change the colours the settings window should show: the
//! shared `theme.css` (global wallpaper change / `bread-theme reload`); the
//! `palettes/<output>.json` + `themes/<output>.css` for the monitor this
//! window is on (`breadpaper set_on` on any monitor, focused or not — which
//! never rewrites `theme.css`); and the window being dragged onto a
//! different monitor. The directory watch is recursive so it catches the
//! first two; a `WindowEvent::Moved` handler catches the third.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// Initial theme fetch — called once by the frontend at startup.
#[tauri::command]
pub fn get_theme_css(window: tauri::WebviewWindow) -> String {
    render_theme_css(&palette_for_window(&window))
}

/// Connector name of the monitor currently showing this window, if known.
fn monitor_name(window: &tauri::WebviewWindow) -> Option<String> {
    window
        .current_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().map(|s| s.to_string()))
}

fn palette_for_window(window: &tauri::WebviewWindow) -> bread_theme::Palette {
    monitor_name(window)
        .map(|name| bread_theme::load_palette_for(&name))
        .unwrap_or_else(bread_theme::load_palette)
}

fn render_theme_css(palette: &bread_theme::Palette) -> String {
    // Use bread-theme's own generators rather than a hand-rolled copy of the
    // `:root { --name: ... }` block. They format the canonical `color_pairs`
    // list every GTK app shares, so the webview accents on the same slot as
    // the rest of the desktop:
    //   --accent  = color4  (was color1 here — pywal's ANSI red, the slot
    //                        reserved for errors; made --accent == --red ==
    //                        --on-accent == --on-red)
    //   --red     = color1  (now distinct from --accent again)
    // `css_tokens` additionally fixes the font-family list, which the local
    // copy emitted as one quoted family (`'Varela Round, sans-serif'`),
    // silently dropping the generic fallback.
    format!(
        "{}\n{}",
        bread_theme::css_custom_properties(palette),
        bread_theme::css_tokens()
    )
}

/// Render the current monitor's palette and push it to the webview.
fn emit_current_theme(app: &AppHandle) {
    let css = app
        .get_webview_window("main")
        .map(|w| render_theme_css(&palette_for_window(&w)))
        .unwrap_or_else(|| render_theme_css(&bread_theme::load_palette()));
    let _ = app.emit("theme-changed", css);
}

/// True when a changed path is one this window should recolour for: the
/// shared `theme.css`, or the per-output palette/CSS for the monitor the
/// window is currently on. When the monitor can't be resolved, fall back to
/// reacting to any per-output `*.json` / `*.css` write so a change is never
/// missed.
fn is_relevant_theme_path(path: &Path, shared: &Path, app: &AppHandle) -> bool {
    if path == shared {
        return true;
    }
    match app
        .get_webview_window("main")
        .and_then(|w| monitor_name(&w))
    {
        Some(name) => {
            path == bread_theme::output_palette_path(&name)
                || path == bread_theme::output_css_path(&name)
        }
        None => {
            let ext = path.extension().and_then(|e| e.to_str());
            matches!(ext, Some("json") | Some("css"))
                && path.parent().is_some_and(|d| {
                    d == bread_theme::palettes_dir() || d == bread_theme::themes_dir()
                })
        }
    }
}

/// Watch the generated-theme directory tree and emit `theme-changed` with the
/// freshly rendered CSS whenever the shared stylesheet or this monitor's
/// per-output files are rewritten. Also re-renders when the window is dragged
/// to another monitor. Call once from `setup`.
pub fn watch_and_emit(app: &AppHandle) {
    let shared = bread_theme::shared_css_path();
    let Some(dir) = shared.parent().map(Path::to_path_buf) else {
        return;
    };
    // The dirs must exist to be watched; `bread-theme generate` makes them at
    // login, but create them here too so a settings window started first
    // still arms the watch.
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::create_dir_all(bread_theme::palettes_dir());
    let _ = std::fs::create_dir_all(bread_theme::themes_dir());

    let app_for_watcher = app.clone();
    let shared_for_watcher = shared.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            // Rewrites land as CREATE / MODIFY / RENAME events; the directory
            // watch also sees unrelated siblings, so filter by path.
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return;
            }
            let relevant = event
                .paths
                .iter()
                .any(|p| is_relevant_theme_path(p, &shared_for_watcher, &app_for_watcher));
            if relevant {
                emit_current_theme(&app_for_watcher);
            }
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing_or_eprintln(&format!("theme watcher: failed to create: {e}"));
            return;
        }
    };

    // Recursive: `palettes/` and `themes/` are subdirectories of `dir`.
    if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
        tracing_or_eprintln(&format!(
            "theme watcher: failed to watch {}: {e}",
            dir.display()
        ));
        return;
    }

    // Leaked to stay alive for the process lifetime — this app has exactly
    // one theme watcher, created once at startup, never torn down.
    app.manage(WatcherHandle(watcher));

    // Re-render when the window is dragged onto a different monitor.
    // `Moved` fires continuously during a drag, so only act on an actual
    // monitor change.
    if let Some(window) = app.get_webview_window("main") {
        let app_for_move = app.clone();
        let last_monitor = Mutex::new(monitor_name(&window));
        window.on_window_event(move |event| {
            if !matches!(event, tauri::WindowEvent::Moved(_)) {
                return;
            }
            let Some(w) = app_for_move.get_webview_window("main") else {
                return;
            };
            let now = monitor_name(&w);
            let mut guard = last_monitor.lock().unwrap_or_else(|e| e.into_inner());
            if *guard != now {
                *guard = now;
                emit_current_theme(&app_for_move);
            }
        });
    }
}

struct WatcherHandle(#[allow(dead_code)] RecommendedWatcher);

fn tracing_or_eprintln(msg: &str) {
    eprintln!("{msg}");
}
