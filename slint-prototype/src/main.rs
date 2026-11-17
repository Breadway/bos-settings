//! Slint prototype of bos-settings.
//!
//! This is not the shipping app. `bos-settings` (Tauri + Svelte) is unchanged.
//! Run this with `cargo run -p bos-settings-slint` from `slint-prototype/`,
//! or `cargo run --manifest-path slint-prototype/Cargo.toml`.
//!
//! Home and Appearance are live. The rest of the sidebar is the real
//! navigation, shown so the shell matches, and those pages do not write
//! anything.

mod data;
mod icons;
mod theme;

use std::rc::Rc;

slint::include_modules!();

#[cfg(test)]
mod tests {
    #[test]
    fn modules_link() {
        let _ = super::data::Appearance::default();
    }
}

fn main() -> anyhow::Result<()> {
    let window = AppWindow::new()?;
    apply_palette(&window);
    apply_metrics(&window);
    window.set_search_icon(icons::SEARCH.into());
    let home = data::load_home();
    let nav = nav_items(&home);
    window.set_nav_items(slint::ModelRc::new(slint::VecModel::from(nav)));
    apply_home(&window, &home);

    let appearance = data::load_appearance();
    apply_appearance(&window, &appearance);

    let window_weak = window.as_weak();
    window.on_tile_clicked(move |id| {
        if id.as_str() == "appearance" {
            if let Some(window) = window_weak.upgrade() {
                window.set_page("appearance".into());
            }
        }
    });

    let window_weak = window.as_weak();
    window.on_appearance_changed(move || {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let appearance = read_appearance(&window);
        match data::save_appearance(&appearance) {
            Ok(()) => window.set_appearance_status("".into()),
            Err(err) => window.set_appearance_status(err.into()),
        }
    });

    window
        .window()
        .set_size(slint::PhysicalSize::new(1280, 840));

    // Slint has no resize callback, so poll the window size the way the CSS
    // media queries react. Cheap: two property writes only when it changes.
    let metrics = slint::Timer::default();
    let metrics_weak = window.as_weak();
    metrics.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Some(window) = metrics_weak.upgrade() {
                apply_metrics(&window);
            }
        },
    );

    window.run()?;
    Ok(())
}

fn apply_palette(window: &AppWindow) {
    let Some(palette) = theme::load() else { return };
    let theme = window.global::<Theme>();
    theme.set_bg(theme::solid(palette.bg));
    theme.set_surface(theme::solid(palette.surface));
    theme.set_surface_2_soft(theme::solid(theme::mix(
        theme::Rgba {
            r: 0x1f,
            g: 0x22,
            b: 0x26,
        },
        palette.bg,
        0.3,
    )));
    // bg-2 and surface-2 are fixed in frontend/src/lib/styles/app.css; the
    // runtime palette only replaces bg, surface and the accent, so leave the
    // defaults from theme.slint alone.
    theme.set_fg(theme::solid(palette.fg));
    theme.set_muted(theme::with_alpha(palette.fg, 0.58));
    theme.set_faint(theme::with_alpha(palette.fg, 0.38));
    theme.set_accent(theme::solid(palette.accent));
    theme.set_on_accent(theme::solid(palette.on_accent));
    theme.set_red(theme::solid(palette.red));
    theme.set_line(theme::with_alpha(palette.fg, 0.08));
    // The shipping tiles mix the accent over the page background, which on
    // this wallpaper reads as a warm brown rather than a flat translucent wash.
    theme.set_accent_soft(theme::solid(theme::mix(palette.bg, palette.accent, 0.24)));
    theme.set_accent_line(theme::with_alpha(palette.accent, 0.6));
    if font_installed("Outfit") {
        theme.set_display("Outfit".into());
    }
    if font_installed("Varela Round") {
        theme.set_body("Varela Round".into());
    }
}

/// Match app.css's responsive rules: a 64px rail and 32px page gutters under
/// 900px, a 716px content cap above it.
fn apply_metrics(window: &AppWindow) {
    let slint_window = window.window();
    let size = slint_window.size();
    let scale = slint_window.scale_factor();
    let width = size.width as f32 / scale;
    if width < 100.0 {
        // Before the compositor has mapped us there is no useful size.
        return;
    }
    let rail = width <= 900.0;
    let sidebar = if rail { 64.0 } else { 236.0 };
    let content = (width - sidebar - 1.0 - 64.0).clamp(0.0, 716.0);
    if window.get_sidebar_width() != sidebar {
        window.set_sidebar_width(sidebar);
    }
    if window.get_content_width() != content {
        window.set_content_width(content);
    }
    if window.get_rail() != rail {
        window.set_rail(rail);
    }
}

fn font_installed(family: &str) -> bool {
    std::process::Command::new("fc-list")
        .arg(family)
        .output()
        .ok()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

fn apply_home(window: &AppWindow, home: &data::Home) {
    let initial = home
        .account_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    window.set_account_name(home.account_name.clone().into());
    window.set_account_initial(initial.into());
    window.set_account_host(home.hostname.clone().into());
    window.set_greeting(home.greeting.clone().into());
    window.set_lede(home.lede.clone().into());
    let tiles = vec![
        tile("wifi", "Wi-Fi", &home.wifi_detail, "wifi", home.wifi_on),
        tile(
            "bluetooth",
            "Bluetooth",
            &home.bluetooth_detail,
            "bluetooth",
            home.bluetooth_on,
        ),
        tile("night", "Night light", "Off", "moon", false),
        tile("dnd", "Do not disturb", "Off", "bell", false),
        tile("power", "Power mode", "Not available", "zap", false),
        tile("plane", "Airplane mode", "Off", "plane", false),
    ];
    window.set_tiles(slint::ModelRc::new(slint::VecModel::from(tiles)));
    let attention = vec![
        AttentionRow {
            label: "Review disk space".into(),
            hint: "Package cleanup keeps the newest versions. Review totals before deleting.".into(),
            action: "Manage".into(),
            primary: false,
        },
        AttentionRow {
            label: home.updates.clone().into(),
            hint: "System packages, bread apps and firmware.".into(),
            action: "Review".into(),
            primary: true,
        },
        AttentionRow {
            label: "Set up a file backup".into(),
            hint: "System snapshots protect the system. A separate backup protects your personal files.".into(),
            action: "Set up".into(),
            primary: false,
        },
    ];
    window.set_attention(slint::ModelRc::new(slint::VecModel::from(attention)));
    window.set_battery_label(home.battery_percent.clone().into());
    window.set_battery_arc(battery_arc(home.battery_fraction).into());
    window.set_battery_detail(home.battery_detail.clone().into());
    window.set_storage_detail(home.storage_detail.clone().into());
    window.set_storage_used(home.storage_used);
    window.set_home_status(home.notes.join(" ").into());
}

/// SVG arc for the battery ring: a path in a 48-unit box, starting at twelve
/// o'clock and sweeping clockwise for `fraction` of a turn.
fn battery_arc(fraction: f32) -> String {
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return String::new();
    }
    if fraction >= 0.999 {
        // A single arc with identical endpoints draws nothing, so a full ring
        // is two half arcs.
        return "M 24 0 A 24 24 0 1 1 24 48 A 24 24 0 1 1 24 0".into();
    }
    let angle = (-90.0 + 360.0 * fraction).to_radians();
    let x = 24.0 + 24.0 * angle.cos();
    let y = 24.0 + 24.0 * angle.sin();
    let large = if fraction > 0.5 { 1 } else { 0 };
    format!("M 24 0 A 24 24 0 {large} 1 {x:.3} {y:.3}")
}

fn tile(id: &str, title: &str, detail: &str, glyph: &str, on: bool) -> QuickTile {
    QuickTile {
        id: id.into(),
        title: title.into(),
        detail: detail.into(),
        icon: icons::commands(glyph).into(),
        on,
        enabled: false,
    }
}

fn apply_appearance(window: &AppWindow, appearance: &data::Appearance) {
    let (active, active_alpha) = split_border(&appearance.active_border);
    let (inactive, inactive_alpha) = split_border(&appearance.inactive_border);
    window.set_active_border(active);
    window.set_active_border_alpha(active_alpha);
    window.set_inactive_border(inactive);
    window.set_inactive_border_alpha(inactive_alpha);
    window.set_gaps_in(appearance.gaps_in as f32);
    window.set_gaps_out(appearance.gaps_out as f32);
    window.set_border_size(appearance.border_size as f32);
    window.set_rounding(appearance.rounding as f32);
    window.set_blur(appearance.blur_enabled);
    window.set_blur_size(appearance.blur_size as f32);
    window.set_blur_passes(appearance.blur_passes as f32);
    window.set_shadows(appearance.shadow_enabled);
    window.set_shadow_range(appearance.shadow_range as f32);
    window.set_resize_on_border(appearance.resize_on_border);
    window.set_layout_index(if appearance.layout == "master" { 1 } else { 0 });
}

fn read_appearance(window: &AppWindow) -> data::Appearance {
    data::Appearance {
        gaps_in: window.get_gaps_in() as i64,
        gaps_out: window.get_gaps_out() as i64,
        border_size: window.get_border_size() as i64,
        active_border: join_border(window.get_active_border(), window.get_active_border_alpha()),
        inactive_border: join_border(
            window.get_inactive_border(),
            window.get_inactive_border_alpha(),
        ),
        rounding: window.get_rounding() as i64,
        blur_enabled: window.get_blur(),
        blur_size: window.get_blur_size() as i64,
        blur_passes: window.get_blur_passes() as i64,
        shadow_enabled: window.get_shadows(),
        shadow_range: window.get_shadow_range() as i64,
        resize_on_border: window.get_resize_on_border(),
        layout: if window.get_layout_index() == 1 {
            "master".into()
        } else {
            "dwindle".into()
        },
    }
}

/// Hyprland stores borders as "rgba(RRGGBBAA)"; the alpha slider edits only
/// the last byte.
fn split_border(text: &str) -> (slint::Color, f32) {
    let inner = text
        .trim()
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
        .filter(|s| s.len() == 8);
    let Some(inner) = inner else {
        return (slint::Color::from_rgb_u8(0x80, 0x80, 0x80), 255.0);
    };
    let byte = |i: usize| u8::from_str_radix(&inner[i..i + 2], 16).unwrap_or(0x80);
    (
        slint::Color::from_rgb_u8(byte(0), byte(2), byte(4)),
        byte(6) as f32,
    )
}

fn join_border(color: slint::Color, alpha: f32) -> String {
    format!(
        "rgba({:02x}{:02x}{:02x}{:02x})",
        color.red(),
        color.green(),
        color.blue(),
        alpha.clamp(0.0, 255.0) as u8
    )
}

fn nav_items(home: &data::Home) -> Vec<NavItem> {
    // Same order and labels as frontend/src/lib/sidebar.ts. `badge` is the
    // small mark the shipping sidebar draws; "soon" stands in for pages this
    // prototype does not implement.
    let rows = [
        ("home", "Home", "", "home", "", true),
        ("network", "Wi-Fi & network", "CONNECT", "wifi", "", false),
        ("bluetooth", "Bluetooth", "", "bluetooth", "", false),
        ("sharing", "Sharing", "", "share", "NEW", false),
        ("displays", "Displays", "DEVICES", "monitor", "", false),
        ("sound", "Sound", "", "volume", "", false),
        ("power", "Power & battery", "", "battery", "", false),
        ("mouse", "Mouse & touchpad", "", "mouse", "", false),
        ("keyboard", "Keyboard", "", "keyboard", "", false),
        ("printers", "Printers", "", "printer", "", false),
        (
            "appearance",
            "Appearance",
            "PERSONALISE",
            "palette",
            "",
            true,
        ),
        ("desktop", "Desktop & bar", "", "window", "", false),
        ("lock", "Lock screen", "", "lock", "", false),
        ("notifications", "Notifications", "", "bell", "NEW", false),
        ("defaults", "Default apps", "APPS", "grid", "", false),
        ("getapps", "Get apps", "", "package", "NEW", false),
        ("startup", "Startup", "", "power", "", false),
        ("updates", "Updates", "SYSTEM", "download", "", false),
        ("restore", "Backup & restore", "", "history", "", false),
        ("storage", "Storage", "", "drive", "NEW", false),
        ("memory", "Memory", "", "memory", "", false),
        ("privacy", "Privacy & security", "", "shield", "", false),
        ("users", "Users", "", "users", "", false),
        ("datetime", "Date & language", "", "globe", "", false),
        ("a11y", "Accessibility", "", "a11y", "", false),
        ("about", "About", "", "info", "", false),
    ];
    rows.into_iter()
        .map(|(id, label, section, glyph, badge, live)| NavItem {
            id: id.into(),
            label: label.into(),
            section: section.into(),
            icon: icons::commands(glyph).into(),
            badge: if id == "updates" && home.update_count > 0 {
                home.update_count.to_string().into()
            } else {
                badge.into()
            },
            live,
        })
        .collect()
}

#[allow(dead_code)]
fn _keep_rc() -> Rc<()> {
    Rc::new(())
}
