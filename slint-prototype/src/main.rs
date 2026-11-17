//! Slint prototype of bos-settings.
//!
//! This is not the shipping app. `bos-settings` (Tauri + Svelte) is unchanged.
//! Run this with `cargo run -p bos-settings-slint` from `slint-prototype/`,
//! or `cargo run --manifest-path slint-prototype/Cargo.toml`.
//!
//! Home and Appearance are live. The rest of the sidebar is the real
//! navigation, shown so the shell matches, and those pages do not write
//! anything.

mod apps;
mod backup;
mod bluetooth;
mod bread;
mod breadram;
mod breadbug;
mod breadbar;
mod breadbox;
mod breadpad;
mod breadsearch;
mod breadshot;
mod config;
mod lock;
mod network;
mod sound;
mod nightlight;
mod notifications;
mod prefs;
mod data;
mod displays;
mod firewall;
mod extra_icons;
mod icons;
mod ime;
mod pagedata;
mod theme;
mod updates;

use slint::Model;
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
    window.set_nav_items(slint::ModelRc::new(slint::VecModel::from(nav_items(0))));

    // Reading the system state shells out to nmcli, upower and pacman, and
    // decoding wallpapers takes seconds, so none of it runs before first
    // paint. Each group applies as soon as its data lands.
    let load_weak = window.as_weak();
    std::thread::spawn(move || {
        let home = data::load_home();
        let appearance = data::load_appearance();
        let about = data::load_about();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = load_weak.upgrade() else {
                return;
            };
            window.set_nav_items(slint::ModelRc::new(slint::VecModel::from(nav_items(
                home.update_count,
            ))));
            apply_home(&window, &home);
            apply_appearance(&window, &appearance);
            apply_about(&window, &about);
        });
    });

    let extra_weak = window.as_weak();
    std::thread::spawn(move || {
        let extra = data::load_extra();
        let wallpaper = extra.wallpaper.clone();
        let paths: Vec<(String, String)> = extra
            .library
            .iter()
            .map(|item| (item.name.clone(), item.path.clone()))
            .collect();
        let meta_weak = extra_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = meta_weak.upgrade() {
                apply_extra(&window, &extra);
            }
        });
        // Slint image handles are not Send, so decode to pixels here and
        // build the images back on the event loop.
        if let Some(rgba) = wallpaper
            .as_deref()
            .and_then(|path| decode_image(path, 1400, 800))
            .map(|mut rgba| {
                rgba.round_corners(HERO_RADIUS);
                rgba
            })
        {
            let weak = extra_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    window.set_wallpaper(image_from_rgba(rgba));
                }
            });
        }
        let thumbs: Vec<(String, String, Option<Rgba>)> = paths
            .into_iter()
            .map(|(name, path)| {
                let rgba = decode_image(&path, 320, 200).map(|mut rgba| {
                    rgba.round_corners(THUMB_RADIUS);
                    rgba
                });
                (name, path, rgba)
            })
            .collect();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = extra_weak.upgrade() else {
                return;
            };
            let items: Vec<WallpaperThumb> = thumbs
                .into_iter()
                .filter_map(|(name, path, rgba)| {
                    Some(WallpaperThumb {
                        name: name.into(),
                        path: path.into(),
                        image: image_from_rgba(rgba?),
                    })
                })
                .collect();
            window.set_library(slint::ModelRc::new(slint::VecModel::from(items)));
        });
    });

    let window_weak = window.as_weak();
    window.on_tile_clicked(move |id| {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        if id.as_str() == "appearance" {
            window.set_page("appearance".into());
            return;
        }
        let action = id.to_string();
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let result = match action.as_str() {
                "wifi" => {
                    let on = data::load_home().wifi_on;
                    data::set_wifi_radio(!on)
                }
                "bluetooth" => {
                    let on = data::load_home().bluetooth_on;
                    data::set_bluetooth_power(!on)
                }
                "night" => {
                    let mut cfg = crate::nightlight::load();
                    cfg.enabled = !cfg.enabled;
                    crate::nightlight::save(&cfg)
                }
                "dnd" => crate::notifications::toggle("dnd"),
                "power" => {
                    let home = data::load_home();
                    if home.automatic_power {
                        Err("Automatic power mode is on".into())
                    } else {
                        let profiles = ["power-saver", "balanced", "performance"];
                        let index = profiles
                            .iter()
                            .position(|profile| *profile == home.power_mode)
                            .unwrap_or(1);
                        data::set_power_mode(profiles[(index + 1) % profiles.len()])
                    }
                }
                "plane" => {
                    let home = data::load_home();
                    data::set_airplane(!home.airplane_on)
                }
                _ => Ok(()),
            };
            let home = data::load_home();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                apply_home(&window, &home);
                if let Err(error) = result {
                    window.set_home_status(format!("Could not apply that change: {error}").into());
                }
            });
        });
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

    // Same hand-off the shipping app has: `--page appearance` opens on that
    // page instead of Home.
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--page" {
            if let Some(page) = args.next() {
                window.set_page(page.into());
            }
        }
    }

    window
        .window()
        .set_size(slint::PhysicalSize::new(1280, 840));

    // Slint has no resize callback, so poll the window size the way the CSS
    // media queries react. Cheap: two property writes only when it changes.
    let metrics = slint::Timer::default();
    let metrics_weak = window.as_weak();
    metrics.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        move || {
            if let Some(window) = metrics_weak.upgrade() {
                apply_metrics(&window);
            }
        },
    );

    let theme_poll = slint::Timer::default();
    let theme_weak = window.as_weak();
    let mut theme_stamp = theme::stamp();
    theme_poll.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(1000),
        move || {
            let stamp = theme::stamp();
            if stamp == theme_stamp {
                return;
            }
            theme_stamp = stamp;
            if let Some(window) = theme_weak.upgrade() {
                apply_palette(&window);
            }
        },
    );

    let weak = window.as_weak();
    window.on_theme_selected(move |id| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        match data::set_theme(id.as_str()) {
            Ok(()) => {
                window.set_active_theme(id);
                window.set_appearance_status("".into());
            }
            Err(err) => window.set_appearance_status(err.into()),
        }
    });

    let weak = window.as_weak();
    window.on_wallpaper_selected(move |path| {
        if let Some(window) = weak.upgrade() {
            let output = wallpaper_output(&window);
            apply_wallpaper(&window, path.as_str(), output.as_deref());
        }
    });

    let weak = window.as_weak();
    window.on_choose_wallpaper(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        if let Some(path) = data::choose_wallpaper() {
            let output = wallpaper_output(&window);
            apply_wallpaper(&window, &path, output.as_deref());
        }
    });

    let weak = window.as_weak();
    window.on_monitor_selected(move |name| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        window.set_selected_monitor(name.clone());
        show_output_wallpaper(&window, name.as_str());
    });

    let weak = window.as_weak();
    window.on_same_all_changed(move |same| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        window.set_same_on_all(same);
        if same {
            if let Some(path) = data::current_wallpaper().0 {
                show_wallpaper_path(&window, &path);
            }
        } else {
            let selected = window.get_selected_monitor().to_string();
            show_output_wallpaper(&window, &selected);
        }
    });

    let weak = window.as_weak();
    window.on_restart_shell(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let status = match data::restart_shell() {
            Ok(()) => "".to_string(),
            Err(err) => err,
        };
        window.set_appearance_status(status.into());
    });

    let weak = window.as_weak();
    window.on_text_changed(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let sizes = [12, 14, 16, 18];
        let fonts = ["Varela Round", "Inter", "Noto Sans"];
        let size = sizes[(window.get_text_size_index().max(0) as usize).min(3)];
        let font = fonts[(window.get_font_index().max(0) as usize).min(2)];
        let status = match data::save_text(size, font) {
            Ok(()) => String::new(),
            Err(err) => err,
        };
        window.set_appearance_status(status.into());
    });

    let weak = window.as_weak();
    window.on_copy_info(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let text = format!(
            "{} {}\nKernel: {}\nCPU: {}\nGPU: {}\nMemory: {}\nChannel: {}",
            window.get_about_os(),
            window.get_about_version(),
            window.get_about_kernel(),
            window.get_about_cpu(),
            window.get_about_gpu(),
            window.get_about_memory(),
            window.get_about_channel()
        );
        let status = match data::copy_to_clipboard(&text) {
            Ok(()) => "System info copied.".to_string(),
            Err(err) => err,
        };
        window.set_about_status(status.into());
    });

    let weak = window.as_weak();
    window.on_rename_hostname(move |name| {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let status = match data::set_hostname(name.trim()) {
            Ok(()) => String::new(),
            Err(err) => err,
        };
        window.set_about_status(status.into());
        if window.get_about_status().is_empty() {
            window.set_about_hostname(name.trim().into());
        }
    });

    let weak = window.as_weak();
    window.on_run_checks(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        window.set_about_status("Checking…".into());
        let check_weak = window.as_weak();
        std::thread::spawn(move || {
            let checks = data::health_checks();
            let _ = slint::invoke_from_event_loop(move || {
                let Some(window) = check_weak.upgrade() else {
                    return;
                };
                let rows: Vec<AboutCheck> = checks
                    .iter()
                    .map(|check| AboutCheck {
                        name: check.name.clone().into(),
                        ok: check.ok,
                        output: check.output.clone().into(),
                    })
                    .collect();
                window.set_about_checks(slint::ModelRc::new(slint::VecModel::from(rows)));
                window.set_about_status("".into());
            });
        });
    });

    // ---- Pages built from the shared Group/Row pieces -------------------
    // The sidebar owns `page`, so a timer notices a change the way the old
    // router did, loads that page off the UI thread and applies it.
    let loaded = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let page_timer = slint::Timer::default();
    {
        let weak = window.as_weak();
        let loaded = loaded.clone();
        let busy = busy.clone();
        page_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(80),
            move || {
                let Some(window) = weak.upgrade() else {
                    return;
                };
                let page = window.get_page().to_string();
                if page == *loaded.lock().unwrap() || busy.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                if page == "about" && *loaded.lock().unwrap() != "about" {
                    let data = pagedata::load("datetime");
                    window.set_about_blocks(slint::ModelRc::new(slint::VecModel::from(
                        pagedata::to_blocks(data),
                    )));
                }
                if page == "home" || page == "appearance" || page == "about" {
                    *loaded.lock().unwrap() = page.clone();
                    return;
                }
                busy.store(true, std::sync::atomic::Ordering::SeqCst);
                let load_weak = window.as_weak();
                let loaded = loaded.clone();
                let busy = busy.clone();
                std::thread::spawn(move || {
                    let data = pagedata::load(&page);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(window) = load_weak.upgrade() {
                            apply_page(&window, data);
                        }
                        *loaded.lock().unwrap() = page.clone();
                        busy.store(false, std::sync::atomic::Ordering::SeqCst);
                    });
                });
            },
        );
    }

    let act_weak = window.as_weak();
    window.on_page_act(move |block, row, action| {
        let Some(window) = act_weak.upgrade() else {
            return;
        };
        let page = window.get_page().to_string();
        let action = action.to_string();
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let data = pagedata::apply_action(&page, block as usize, row as usize, &action);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    apply_page(&window, data);
                }
            });
        });
    });

    let block_weak = window.as_weak();
    window.on_page_block_act(move |block, action| {
        let Some(window) = block_weak.upgrade() else {
            return;
        };
        let page = window.get_page().to_string();
        let action = action.to_string();
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let data = pagedata::apply_block_action(&page, block as usize, &action);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    apply_page(&window, data);
                }
            });
        });
    });

    let chip_weak = window.as_weak();
    window.on_page_chip(move |block, chip| {
        let Some(window) = chip_weak.upgrade() else {
            return;
        };
        let page = window.get_page().to_string();
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let data = pagedata::select_chip(&page, block as usize, chip as usize);
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    apply_page(&window, data);
                }
            });
        });
    });

    let weak = window.as_weak();
    window.window().on_close_requested(move || {
        let _ = weak.upgrade();
        displays::revert_on_exit();
        slint::CloseRequestResponse::HideWindow
    });
    window.run()?;
    Ok(())
}

fn apply_page(window: &AppWindow, page: pagedata::Page) {
    let pagedata::Page {
        crumb,
        title,
        lede,
        status,
        blocks,
        ..
    } = page;
    window.set_page_crumb(crumb.into());
    window.set_page_title(title.into());
    window.set_page_lede(lede.into());
    window.set_page_status(status.into());
    window.set_page_blocks(slint::ModelRc::new(slint::VecModel::from(
        pagedata::spec_blocks(blocks),
    )));
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
    // app.css: color-mix(in oklab, accent 18%, transparent). Keep it
    // translucent so it picks up each surface it sits on (page, sidebar,
    // cards) the way the shipping app does.
    theme.set_accent_soft(theme::with_alpha(palette.accent, 0.18));
    theme.set_accent_line(theme::with_alpha(palette.accent, 0.55));
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

const TEXT_SIZES: [i64; 4] = [12, 14, 16, 18];
const FONTS: [&str; 3] = ["Varela Round", "Inter", "Noto Sans"];

fn apply_about(window: &AppWindow, about: &data::About) {
    window.set_about_os(about.os.clone().into());
    window.set_about_version(about.version.clone().unwrap_or_default().into());
    window.set_about_kernel(about.kernel.clone().into());
    window.set_about_cpu(about.cpu.clone().into());
    window.set_about_gpu(about.gpu.clone().into());
    window.set_about_gpu_full(about.gpu_full.clone().into());
    window.set_about_memory(about.memory.clone().into());
    window.set_about_disk(about.disk.clone().into());
    window.set_about_uptime(about.uptime.clone().into());
    window.set_about_hostname(about.hostname.clone().into());
    window.set_about_channel(capitalize(&about.channel).into());
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn apply_extra(window: &AppWindow, extra: &data::Extra) {
    let cards: Vec<ThemeCard> = extra
        .themes
        .iter()
        .map(|theme| ThemeCard {
            id: theme.id.clone().into(),
            name: theme.name.clone().into(),
            preview: theme.preview.clone().into(),
            sub: if theme.custom {
                "Custom theme".into()
            } else {
                "".into()
            },
        })
        .collect();
    window.set_themes(slint::ModelRc::new(slint::VecModel::from(cards)));
    window.set_active_theme(extra.active_theme.clone().into());
    window.set_wallpaper_name(extra.wallpaper_name.clone().into());
    window.set_wallpaper_path(extra.wallpaper.clone().unwrap_or_default().into());
    window.set_library_dir(extra.library_dir.clone().into());
    let monitors: Vec<MonitorOption> = extra
        .monitors
        .iter()
        .map(|monitor| MonitorOption {
            name: monitor.name.clone().into(),
            mode: monitor.mode.clone().into(),
        })
        .collect();
    let selected = extra
        .monitors
        .first()
        .map(|monitor| monitor.name.clone())
        .unwrap_or_default();
    let distinct: std::collections::HashSet<&str> = extra
        .wallpapers
        .iter()
        .map(|(_, path)| path.as_str())
        .collect();
    window.set_same_on_all(distinct.len() <= 1);
    window.set_selected_monitor(selected.clone().into());
    window.set_monitors(slint::ModelRc::new(slint::VecModel::from(monitors)));
    // Filled in once the thumbnails have decoded; empty cells would just
    // flicker transparent.
    window.set_library(slint::ModelRc::new(slint::VecModel::from(Vec::<
        WallpaperThumb,
    >::new())));
    window.set_text_size_index(
        TEXT_SIZES
            .iter()
            .position(|size| *size == extra.text_size)
            .unwrap_or(1) as i32,
    );
    window.set_font_index(
        FONTS
            .iter()
            .position(|font| *font == extra.font)
            .unwrap_or(0) as i32,
    );
}

struct Rgba {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl Rgba {
    /// Zero the alpha outside a rounded rectangle, since Slint clips to the
    /// bounding box and would otherwise square off a rounded Image.
    fn round_corners(&mut self, radius: f32) {
        let w = self.width as f32;
        let h = self.height as f32;
        let r = radius.max(0.0).min(w / 2.0).min(h / 2.0);
        if r <= 0.0 {
            return;
        }
        for y in 0..self.height {
            for x in 0..self.width {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let cx = if px < r {
                    r
                } else if px > w - r {
                    w - r
                } else {
                    px
                };
                let cy = if py < r {
                    r
                } else if py > h - r {
                    h - r
                } else {
                    py
                };
                let dx = px - cx;
                let dy = py - cy;
                if dx * dx + dy * dy > r * r {
                    let i = ((y * self.width + x) * 4 + 3) as usize;
                    self.pixels[i] = 0;
                }
            }
        }
    }
}

fn decode_image(path: &str, max_width: u32, max_height: u32) -> Option<Rgba> {
    let image = image::open(path).ok()?;
    let image = image.thumbnail(max_width, max_height);
    let rgba = image.to_rgba8();
    Some(Rgba {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

/// Radii are in source pixels: the hero shows 1400px at about 684 wide
/// (12px on screen) and a thumb shows 320 at about 130 (8px on screen).
const HERO_RADIUS: f32 = 24.0;
const THUMB_RADIUS: f32 = 20.0;

fn image_from_rgba(rgba: Rgba) -> slint::Image {
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        &rgba.pixels,
        rgba.width,
        rgba.height,
    );
    slint::Image::from_rgba8(buffer)
}

fn wallpaper_output(window: &AppWindow) -> Option<String> {
    let monitors = window.get_monitors();
    let selected = window.get_selected_monitor().to_string();
    (!window.get_same_on_all() && monitors.row_count() > 1 && !selected.is_empty())
        .then_some(selected)
}

fn apply_wallpaper(window: &AppWindow, path: &str, output: Option<&str>) {
    let result = match output {
        Some(output) => data::set_wallpaper_on(output, path),
        None => data::set_wallpaper(path),
    };
    match result {
        Ok(()) => {
            show_wallpaper_path(window, path);
            window.set_appearance_status("".into());
        }
        Err(err) => window.set_appearance_status(err.into()),
    }
}

fn show_wallpaper_path(window: &AppWindow, path: &str) {
    let name = path.rsplit('/').next().unwrap_or("").to_string();
    window.set_wallpaper_path(path.into());
    window.set_wallpaper_name(name.into());
    window.set_wallpaper(
        load_image(path, 1400, 800)
            .map(|mut rgba| {
                rgba.round_corners(HERO_RADIUS);
                image_from_rgba(rgba)
            })
            .unwrap_or_else(slint::Image::default),
    );
}

fn show_output_wallpaper(window: &AppWindow, output: &str) {
    let path = data::wallpapers_by_output()
        .into_iter()
        .find(|(name, _)| name == output)
        .map(|(_, path)| path)
        .or_else(|| data::current_wallpaper().0);
    if let Some(path) = path {
        show_wallpaper_path(window, &path);
    }
}

/// Decode a wallpaper and downscale it before handing it to Slint; full-size
/// 4K images are tens of megabytes each and the page shows a dozen of them.
fn load_image(path: &str, max_width: u32, max_height: u32) -> Option<Rgba> {
    decode_image(path, max_width, max_height)
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
    let night_detail = if home.night_installed {
        if home.night_on { "On" } else { "Off" }
    } else {
        "Not installed"
    };
    let power_detail = if home.automatic_power {
        format!("{} · Auto", home.power_mode_detail)
    } else {
        home.power_mode_detail.clone()
    };
    let plane_detail = if home.airplane_available {
        if home.airplane_on { "Radios off" } else { "Off" }
    } else {
        "No radios found"
    };
    let tiles = vec![
        tile("wifi", "Wi-Fi", &home.wifi_detail, "wifi", home.wifi_on),
        tile(
            "bluetooth",
            "Bluetooth",
            &home.bluetooth_detail,
            "bluetooth",
            home.bluetooth_on,
        ),
        tile_enabled(
            "night",
            "Night light",
            night_detail,
            "moon",
            home.night_on,
            home.night_installed,
        ),
        tile("dnd", "Do not disturb", &home.dnd_detail, "bell", home.dnd_on),
        tile_enabled(
            "power",
            "Power mode",
            &power_detail,
            "zap",
            false,
            !home.power_mode.is_empty() && !home.automatic_power,
        ),
        tile_enabled(
            "plane",
            "Airplane mode",
            plane_detail,
            "plane",
            home.airplane_on,
            home.airplane_available,
        ),
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
            label: if home.update_count > 0 {
                format!("{} updates ready", home.update_count).into()
            } else {
                "No package updates found".into()
            },
            hint: "System packages, bread apps and firmware.".into(),
            action: "Review".into(),
            primary: true,
        },
        AttentionRow {
            label: if home.backup_ready {
                "Your file backup".into()
            } else {
                "Set up a file backup".into()
            },
            hint: "System snapshots protect the system. A separate backup protects your personal files.".into(),
            action: if home.backup_ready { "Review".into() } else { "Set up".into() },
            primary: false,
        },
    ];
    window.set_attention(slint::ModelRc::new(slint::VecModel::from(attention)));
    window.set_battery_label(home.battery_percent.clone().into());
    window.set_battery_arc(battery_arc(home.battery_fraction).into());
    window.set_battery_detail(home.battery_detail.clone().into());
    window.set_storage_detail(home.storage_detail.clone().into());
    window.set_storage_used(home.storage_used);
    let mut status = home.notes.join(" ");
    if !home.connected.is_empty() {
        if !status.is_empty() {
            status.push(' ');
        }
        status.push_str(&format!("Connected: {}.", home.connected));
    }
    window.set_home_status(status.into());
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
    tile_enabled(id, title, detail, glyph, on, true)
}

fn tile_enabled(
    id: &str,
    title: &str,
    detail: &str,
    glyph: &str,
    on: bool,
    enabled: bool,
) -> QuickTile {
    QuickTile {
        id: id.into(),
        title: title.into(),
        detail: detail.into(),
        icon: icons::commands(glyph).into(),
        on,
        enabled,
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

fn nav_items(update_count: usize) -> Vec<NavItem> {
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
            badge: if id == "updates" && update_count > 0 {
                update_count.to_string().into()
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
