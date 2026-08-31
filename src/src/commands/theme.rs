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
pub fn get_theme_css(window: tauri::WebviewWindow) -> String {
    render_theme_css(&palette_for_window(&window))
}

fn palette_for_window(window: &tauri::WebviewWindow) -> bread_theme::Palette {
    window
        .current_monitor()
        .ok()
        .flatten()
        .and_then(|m| m.name().map(|s| s.to_string()))
        .map(|name| bread_theme::load_palette_for(&name))
        .unwrap_or_else(bread_theme::load_palette)
}

fn render_theme_css(palette: &bread_theme::Palette) -> String {
    // bread-theme v0.7.1 exposes Palette + ink_on + tokens, but not the
    // later css_custom_properties / css_tokens helpers (those landed after
    // the tag). Emit the same :root custom-property names the Svelte app
    // already uses so a tag pin doesn't require a web-side rename.
    format!("{}\n{}", css_custom_properties(palette), css_tokens())
}

fn css_custom_properties(p: &bread_theme::Palette) -> String {
    let pairs = [
        ("bg", p.background.as_str()),
        ("fg", p.foreground.as_str()),
        ("surface", p.color0.as_str()),
        ("overlay", p.color7.as_str()),
        ("accent", p.color1.as_str()),
        ("red", p.color1.as_str()),
        ("green", p.color2.as_str()),
        ("yellow", p.color3.as_str()),
        ("blue", p.color4.as_str()),
        ("pink", p.color5.as_str()),
        ("teal", p.color6.as_str()),
        ("on-bg", bread_theme::ink_on(&p.background)),
        ("on-surface", bread_theme::ink_on(&p.color0)),
        ("on-accent", bread_theme::ink_on(&p.color1)),
        ("on-red", bread_theme::ink_on(&p.color1)),
        ("on-overlay", bread_theme::ink_on(&p.color7)),
    ];
    let vars: String = pairs
        .iter()
        .map(|(name, value)| format!("  --{name}: {value};\n"))
        .collect();
    format!(":root {{\n{vars}}}\n")
}

fn css_tokens() -> String {
    use bread_theme::tokens::*;
    format!(
        ":root {{\n\
         \x20\x20--font-family: '{font}';\n\
         \x20\x20--font-size-base: {base}px;\n\
         \x20\x20--font-size-secondary: {sec}px;\n\
         \x20\x20--space-xs: {xs}px;\n\
         \x20\x20--space-sm: {sm}px;\n\
         \x20\x20--space-md: {md}px;\n\
         \x20\x20--space-lg: {lg}px;\n\
         \x20\x20--space-xl: {xl}px;\n\
         \x20\x20--radius-primary: {r1}px;\n\
         \x20\x20--radius-secondary: {r2}px;\n\
         \x20\x20--radius-tertiary: {r3}px;\n\
         \x20\x20--radius-pill: {pill}px;\n\
         }}\n",
        font = FONT_FAMILY,
        base = FONT_SIZE_BASE,
        sec = FONT_SIZE_SECONDARY,
        xs = SPACE_XS,
        sm = SPACE_SM,
        md = SPACE_MD,
        lg = SPACE_LG,
        xl = SPACE_XL,
        r1 = RADIUS_PRIMARY,
        r2 = RADIUS_SECONDARY,
        r3 = RADIUS_TERTIARY,
        pill = RADIUS_PILL,
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
                let css = app_for_watcher
                    .get_webview_window("main")
                    .map(|w| render_theme_css(&palette_for_window(&w)))
                    .unwrap_or_else(|| render_theme_css(&bread_theme::load_palette()));
                let _ = app_for_watcher.emit("theme-changed", css);
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
        tracing_or_eprintln(&format!(
            "theme watcher: failed to watch {}: {e}",
            dir.display()
        ));
        return;
    }

    // Leaked to stay alive for the process lifetime — this app has exactly
    // one theme watcher, created once at startup, never torn down.
    app.manage(WatcherHandle(watcher));
}

struct WatcherHandle(#[allow(dead_code)] RecommendedWatcher);

fn tracing_or_eprintln(msg: &str) {
    eprintln!("{msg}");
}
