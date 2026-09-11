//! Bridges `bread-theme`'s pywal-derived palette into the webview as CSS
//! custom properties, and keeps it live.
//!
//! `bread-theme` rewrites every file it generates with a temp-then-rename
//! (atomic replace), which kills a direct file watch (inotify reports
//! `DELETE_SELF` and never re-arms), so this watches the *parent directory*
//! and filters by path instead, the same strategy `bread_theme::gtk`'s
//! `watch_theme_file` / `ensure_themes_watch` use for the GTK apps.
//!
//! Three inputs can change the colours the settings window should show: the
//! shared `theme.css` (global wallpaper change / `bread-theme reload`); the
//! `palettes/<output>.json` + `themes/<output>.css` for the monitor this
//! window is on (`breadpaper set_on` on any monitor, focused or not, which
//! never rewrites `theme.css`); and the window being dragged onto a
//! different monitor. The directory watch is recursive so it catches the
//! first two; a `WindowEvent::Moved` handler catches the third.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use super::hyprland::LiveMonitor;

/// Initial theme fetch; called once by the frontend at startup.
#[tauri::command]
pub fn get_theme_css(window: tauri::WebviewWindow) -> String {
    render_theme_css(&palette_for_window(&window))
}

/// Hyprland connector (`"eDP-1"`, `"DP-2"`, ...) of the monitor currently
/// showing `window`, if it can be resolved.
///
/// `tauri::Monitor::name()` does not return the Hyprland connector on
/// Wayland: it comes back `None`, or a model string with no `palettes/*.json`
/// behind it (see BSET-10). Its *geometry* (physical position, size, scale),
/// however, is populated straight from the compositor (the `wl_output` bound
/// for this surface's `wl_surface.enter`), so match that against `hyprctl
/// monitors -j`'s per-connector geometry instead of trusting the name.
fn resolve_connector(window: &tauri::WebviewWindow) -> Option<String> {
    let monitor = window.current_monitor().ok().flatten()?;
    let live = super::hyprland::get_live_monitors();
    connector_for_geometry(
        (monitor.position().x, monitor.position().y),
        (monitor.size().width, monitor.size().height),
        &live,
    )
}

/// Pure matcher behind [`resolve_connector`], kept separate so it can be unit
/// tested without a real window or a `hyprctl` on `PATH`.
fn connector_for_geometry(
    position: (i32, i32),
    size: (u32, u32),
    live: &[LiveMonitor],
) -> Option<String> {
    if live.is_empty() {
        return None;
    }
    // Physical resolution is scale-independent and reported identically by
    // both APIs, so it is by far the most reliable key: filter to it first.
    let same_size: Vec<&LiveMonitor> = live
        .iter()
        .filter(|m| m.width == size.0 && m.height == size.1)
        .collect();
    let candidates: Vec<&LiveMonitor> = if same_size.is_empty() {
        // Nothing matched on resolution (e.g. Tauri's cached monitor list is
        // momentarily stale after a mode change): fall back to considering
        // every live output rather than giving up outright.
        live.iter().collect()
    } else {
        same_size
    };
    if candidates.len() == 1 {
        return Some(candidates[0].name.clone());
    }
    // Multiple same-resolution candidates (identical monitor models, or the
    // size fallback above): disambiguate by position. Hyprland's `x`/`y` are
    // logical layout coordinates; Tauri's position is physical, so scale each
    // candidate's coordinates by its own scale before comparing distances.
    candidates
        .into_iter()
        .min_by(|a, b| {
            let da = position_distance(position, a);
            let db = position_distance(position, b);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|m| m.name.clone())
}

fn position_distance(position: (i32, i32), m: &LiveMonitor) -> f64 {
    let mx = m.x as f64 * m.scale;
    let my = m.y as f64 * m.scale;
    let dx = position.0 as f64 - mx;
    let dy = position.1 as f64 - my;
    (dx * dx + dy * dy).sqrt()
}

/// Cheap (no `hyprctl` subprocess) monitor identity, used only to detect that
/// a window actually crossed onto a different output. `WindowEvent::Moved`
/// fires continuously while a window is being dragged, so the per-event check
/// must not shell out; full connector resolution only happens once an actual
/// change is confirmed (see `watch_and_emit`).
fn monitor_geometry(window: &tauri::WebviewWindow) -> Option<((i32, i32), (u32, u32))> {
    let m = window.current_monitor().ok().flatten()?;
    Some((
        (m.position().x, m.position().y),
        (m.size().width, m.size().height),
    ))
}

fn palette_for_window(window: &tauri::WebviewWindow) -> bread_theme::Palette {
    resolve_connector(window)
        .map(|name| bread_theme::load_palette_for(&name))
        .unwrap_or_else(bread_theme::load_palette)
}

fn render_theme_css(palette: &bread_theme::Palette) -> String {
    // Use bread-theme's own generators rather than a hand-rolled copy of the
    // `:root { --name: ... }` block. They format the canonical `color_pairs`
    // list every GTK app shares, so the webview accents on the same slot as
    // the rest of the desktop:
    //   --accent  = color4  (was color1 here, pywal's ANSI red, the slot
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
        .and_then(|w| resolve_connector(&w))
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

    // Leaked to stay alive for the process lifetime; this app has exactly
    // one theme watcher, created once at startup, never torn down.
    app.manage(WatcherHandle(watcher));

    // Re-render when the window is dragged onto a different monitor.
    // `Moved` fires continuously during a drag, so only act on an actual
    // monitor change.
    if let Some(window) = app.get_webview_window("main") {
        let app_for_move = app.clone();
        let last_geometry = Mutex::new(monitor_geometry(&window));
        window.on_window_event(move |event| {
            if !matches!(event, tauri::WindowEvent::Moved(_)) {
                return;
            }
            let Some(w) = app_for_move.get_webview_window("main") else {
                return;
            };
            let now = monitor_geometry(&w);
            let mut guard = last_geometry.lock().unwrap_or_else(|e| e.into_inner());
            if *guard != now {
                *guard = now;
                // Geometry changed, i.e. the window is now on a different
                // output: re-render, which resolves the new connector.
                emit_current_theme(&app_for_move);
            }
        });
    }
}

struct WatcherHandle(#[allow(dead_code)] RecommendedWatcher);

fn tracing_or_eprintln(msg: &str) {
    eprintln!("{msg}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, y: i32, width: u32, height: u32, scale: f64) -> LiveMonitor {
        LiveMonitor {
            name: name.to_string(),
            mode: format!("{width}x{height}"),
            x,
            y,
            width,
            height,
            refresh: 60.0,
            scale,
            transform: 0,
            available_modes: Vec::new(),
        }
    }

    #[test]
    fn connector_for_geometry_empty_live_list_is_none() {
        assert_eq!(connector_for_geometry((0, 0), (1920, 1080), &[]), None);
    }

    #[test]
    fn connector_for_geometry_matches_unique_resolution() {
        let live = [
            monitor("eDP-1", 0, 0, 1920, 1200, 1.0),
            monitor("HDMI-A-1", 1920, 0, 2560, 1440, 1.0),
        ];
        // Position is deliberately way off; resolution alone is unambiguous
        // here, so it should still resolve correctly.
        assert_eq!(
            connector_for_geometry((5000, 5000), (2560, 1440), &live),
            Some("HDMI-A-1".to_string())
        );
        assert_eq!(
            connector_for_geometry((0, 0), (1920, 1200), &live),
            Some("eDP-1".to_string())
        );
    }

    #[test]
    fn connector_for_geometry_disambiguates_same_resolution_by_position() {
        // Two identical-model externals side by side, both 2560x1440 at
        // scale 1: physical position equals logical position here.
        let live = [
            monitor("DP-1", 0, 0, 2560, 1440, 1.0),
            monitor("DP-2", 2560, 0, 2560, 1440, 1.0),
        ];
        assert_eq!(
            connector_for_geometry((100, 100), (2560, 1440), &live),
            Some("DP-1".to_string())
        );
        assert_eq!(
            connector_for_geometry((2700, 100), (2560, 1440), &live),
            Some("DP-2".to_string())
        );
    }

    #[test]
    fn connector_for_geometry_scales_position_before_comparing() {
        // A single fractionally-scaled output: Hyprland's x/y are logical,
        // Tauri's position is physical (logical * scale).
        let live = [monitor("eDP-1", 100, 50, 1920, 1200, 1.5)];
        assert_eq!(
            connector_for_geometry((150, 75), (1920, 1200), &live),
            Some("eDP-1".to_string())
        );
    }

    #[test]
    fn connector_for_geometry_falls_back_to_all_outputs_on_resolution_miss() {
        // No live output reports this exact resolution (e.g. a stale cache);
        // still resolve to the geometrically closest one rather than giving
        // up.
        let live = [
            monitor("eDP-1", 0, 0, 1920, 1200, 1.0),
            monitor("HDMI-A-1", 3000, 0, 2560, 1440, 1.0),
        ];
        assert_eq!(
            connector_for_geometry((10, 10), (1921, 1201), &live),
            Some("eDP-1".to_string())
        );
    }
}
