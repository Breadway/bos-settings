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
    window.set_search_icon(icon("search"));
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

    window.run()?;
    Ok(())
}

fn apply_palette(window: &AppWindow) {
    let Some(palette) = theme::load() else { return };
    let theme = window.global::<Theme>();
    theme.set_bg(theme::solid(palette.bg));
    theme.set_bg_2(theme::solid(theme::mix(palette.bg, palette.surface, 0.55)));
    theme.set_surface(theme::solid(palette.surface));
    theme.set_surface_2(theme::solid(theme::mix(palette.surface, palette.fg, 0.06)));
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
        tile(
            "wifi",
            "Wi-Fi",
            &home.wifi_detail,
            icon("wifi"),
            home.wifi_on,
        ),
        tile(
            "bluetooth",
            "Bluetooth",
            &home.bluetooth_detail,
            icon("bluetooth"),
            home.bluetooth_on,
        ),
        tile("night", "Night light", "Off", icon("moon"), false),
        tile("dnd", "Do not disturb", "Off", icon("bell"), false),
        tile("power", "Power mode", "Not available", icon("zap"), false),
        tile("plane", "Airplane mode", "Off", icon("plane"), false),
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
    window.set_battery_ring(battery_ring(home.battery_fraction, &window));
    window.set_battery_detail(home.battery_detail.clone().into());
    window.set_storage_detail(home.storage_detail.clone().into());
    window.set_storage_used(home.storage_used);
    window.set_home_status(home.notes.join(" ").into());
}

/// A thin ring: a faint full track, and an accent arc from 12 o'clock that
/// covers `fraction` of the circle. Drawn here because a conic gradient
/// filled its whole box and the hole punched in it never showed.
fn battery_ring(fraction: f32, window: &AppWindow) -> slint::Image {
    let theme = window.global::<Theme>();
    let accent = theme.get_accent();
    let track = theme.get_surface_2();
    let size = 96u32;
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let mut paint = tiny_skia::Paint::default();
    let center = size as f32 / 2.0;
    let radius = 40.0;
    let stroke = tiny_skia::Stroke {
        width: 8.0,
        ..tiny_skia::Stroke::default()
    };
    let circle = ring_path(center, radius, 0.0, 360.0);
    paint.set_color_rgba8(track.red(), track.green(), track.blue(), 255);
    pixmap.stroke_path(
        &circle,
        &paint,
        &stroke,
        tiny_skia::Transform::identity(),
        None,
    );
    let sweep = (fraction.clamp(0.0, 1.0) * 360.0).max(1.0);
    let path = ring_path(center, radius, -90.0, sweep);
    {
        paint.set_color_rgba8(accent.red(), accent.green(), accent.blue(), 255);
        paint.anti_alias = true;
        let arc_stroke = tiny_skia::Stroke {
            width: 8.0,
            line_cap: tiny_skia::LineCap::Round,
            ..stroke
        };
        pixmap.stroke_path(
            &path,
            &paint,
            &arc_stroke,
            tiny_skia::Transform::identity(),
            None,
        );
    }
    let buffer =
        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(pixmap.data(), size, size);
    slint::Image::from_rgba8(buffer)
}

/// An arc of `sweep` degrees starting at `start`. 0 is 3 o'clock and positive
/// is clockwise, so -90 is 12 o'clock. Built from short lines because this
/// version of tiny-skia has no arc helper.
fn ring_path(center: f32, radius: f32, start: f32, sweep: f32) -> tiny_skia::Path {
    let steps = (sweep.abs() / 4.0).ceil().max(1.0) as i32;
    let mut path = tiny_skia::PathBuilder::new();
    for step in 0..=steps {
        let angle = (start + sweep * step as f32 / steps as f32).to_radians();
        let (x, y) = (center + radius * angle.cos(), center + radius * angle.sin());
        if step == 0 {
            path.move_to(x, y);
        } else {
            path.line_to(x, y);
        }
    }
    path.finish().expect("ring path")
}

fn tile(id: &str, title: &str, detail: &str, image: slint::Image, on: bool) -> QuickTile {
    QuickTile {
        id: id.into(),
        title: title.into(),
        detail: detail.into(),
        icon: image,
        on,
        enabled: false,
    }
}

fn icon(name: &str) -> slint::Image {
    // The bytes are compiled in, then unpacked once, because Slint loads an
    // image from a path. Reading the source tree broke the copy on Hermes.
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|d| std::path::PathBuf::from(d).join("bos-settings-slint/icons"))
        .unwrap_or_else(|| std::env::temp_dir().join("bos-settings-slint/icons"));
    let path = dir.join(format!("{name}.png"));
    if !path.exists() {
        if let Some(bytes) = icon_bytes(name) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&path, bytes);
        }
    }
    slint::Image::load_from_path(&path).unwrap_or_default()
}

fn icon_bytes(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "home" => include_bytes!("../ui/icons/home.png"),
        "wifi" => include_bytes!("../ui/icons/wifi.png"),
        "bluetooth" => include_bytes!("../ui/icons/bluetooth.png"),
        "share" => include_bytes!("../ui/icons/share.png"),
        "monitor" => include_bytes!("../ui/icons/monitor.png"),
        "volume" => include_bytes!("../ui/icons/volume.png"),
        "battery" => include_bytes!("../ui/icons/battery.png"),
        "mouse" => include_bytes!("../ui/icons/mouse.png"),
        "keyboard" => include_bytes!("../ui/icons/keyboard.png"),
        "printer" => include_bytes!("../ui/icons/printer.png"),
        "palette" => include_bytes!("../ui/icons/palette.png"),
        "window" => include_bytes!("../ui/icons/window.png"),
        "lock" => include_bytes!("../ui/icons/lock.png"),
        "bell" => include_bytes!("../ui/icons/bell.png"),
        "grid" => include_bytes!("../ui/icons/grid.png"),
        "package" => include_bytes!("../ui/icons/package.png"),
        "power" => include_bytes!("../ui/icons/power.png"),
        "download" => include_bytes!("../ui/icons/download.png"),
        "history" => include_bytes!("../ui/icons/history.png"),
        "drive" => include_bytes!("../ui/icons/drive.png"),
        "memory" => include_bytes!("../ui/icons/memory.png"),
        "shield" => include_bytes!("../ui/icons/shield.png"),
        "users" => include_bytes!("../ui/icons/users.png"),
        "globe" => include_bytes!("../ui/icons/globe.png"),
        "a11y" => include_bytes!("../ui/icons/a11y.png"),
        "info" => include_bytes!("../ui/icons/info.png"),
        "search" => include_bytes!("../ui/icons/search.png"),
        "moon" => include_bytes!("../ui/icons/moon.png"),
        "zap" => include_bytes!("../ui/icons/zap.png"),
        "plane" => include_bytes!("../ui/icons/plane.png"),
        _ => return None,
    })
}

fn apply_appearance(window: &AppWindow, appearance: &data::Appearance) {
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
            icon: icon(glyph),
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
