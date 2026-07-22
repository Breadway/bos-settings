//! Bridges `bread-theme`'s pywal-derived palette into the webview as CSS
//! custom properties, and keeps it live: `bread-theme`'s generator rewrites
//! its shared stylesheet with a temp-then-rename (atomic replace), which
//! kills a direct file watch (inotify reports DELETE_SELF and never
//! re-arms) — so this watches the *parent directory* and filters by
//! filename instead, the same strategy `bread_theme::gtk::watch_theme_file`
//! uses for the GTK apps.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};

/// Initial theme fetch — called once by the frontend at startup.
#[tauri::command]
pub fn get_theme_css() -> String {
    render_theme_css()
}

fn render_theme_css() -> String {
    let palette = bread_theme::load_palette();
    format!(
        "{}\n{}",
        bread_theme::css_custom_properties(&palette),
        bread_theme::css_tokens(),
    )
}

/// Start watching the shared theme file and emit `theme-changed` with the
/// freshly rendered CSS whenever it's rewritten (palette change from a new
/// wallpaper, or a manual `bread-theme reload`). Call once from `setup`.
pub fn watch_and_emit(app: &AppHandle) {
    let target = bread_theme::shared_css_path();
    let Some(dir) = target.parent() else { return };
    let _ = std::fs::create_dir_all(dir);

    let app_for_watcher = app.clone();
    let target_for_watcher = target.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            let Ok(event) = res else { return };
            // Rewrites land as CREATE/MODIFY/RENAME events touching the
            // stylesheet's path specifically — the directory watch also
            // sees unrelated siblings, so filter to the target file.
            let touches_target = matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) && event.paths.iter().any(|p| p == &target_for_watcher);
            if touches_target {
                let _ = app_for_watcher.emit("theme-changed", render_theme_css());
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

    if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
        tracing_or_eprintln(&format!("theme watcher: failed to watch {}: {e}", dir.display()));
        return;
    }

    // Leaked to stay alive for the process lifetime — this app has exactly
    // one theme watcher, created once at startup, never torn down.
    app.manage(WatcherHandle(watcher));
}

struct WatcherHandle(RecommendedWatcher);

fn tracing_or_eprintln(msg: &str) {
    eprintln!("{msg}");
}
