//! breadpaper — wallpaper manager. No config file to edit here; breadpaper
//! takes no persistent settings, just an image path via its CLI (`breadpaper
//! set <path>` / `breadpaper get`). This panel is a thin GUI front-end for
//! that CLI so wallpaper (and the pywal-driven theme it generates) has a
//! discoverable home in BOS Settings instead of only being reachable from a
//! terminal.

use std::path::{Path, PathBuf};
use std::process::Command;

use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, FileDialog, FileFilter, FlowBox, Image, Label, Orientation, Picture,
};

use crate::ui::widgets as w;

/// Extensions breadpaper's own `validate()` accepts — narrower than what
/// GdkPixbuf can decode (svg/tiff/etc.), so both the file picker's filter and
/// the library scan below stay in sync with what `breadpaper set` will
/// actually take instead of offering images it'll reject.
const WALLPAPER_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Caps how many thumbnails the library grid ever builds. Each one decodes a
/// scaled `Pixbuf` synchronously on the main thread (see `thumbnail`) — fine
/// for a bounded, explicitly-triggered scan (same "costs seconds, gated
/// behind a button" posture as network.rs's Wi-Fi scan), not fine as an
/// unbounded walk of someone's whole Pictures folder.
const MAX_LIBRARY_ITEMS: usize = 80;
const MAX_SCAN_DEPTH: usize = 4;

fn wallpaper_library_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join("Pictures/Backgrounds")
}

fn is_wallpaper_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| WALLPAPER_EXTS.iter().any(|ext| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// Recursively collects image paths under `dir` (sorted, depth- and
/// count-bounded) — the library is organized in subfolders (e.g. by show/
/// series), not a flat directory, so a non-recursive scan would find nothing.
fn scan_wallpapers(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        if depth == 0 || out.len() >= MAX_LIBRARY_ITEMS {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            if out.len() >= MAX_LIBRARY_ITEMS {
                return;
            }
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth - 1, out);
            } else if is_wallpaper_file(&path) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, MAX_SCAN_DEPTH, &mut out);
    out
}

/// A clickable thumbnail: scaled-decode preview + filename, wrapped in a
/// plain `Button` so the whole card is the click target. Returns `None` for
/// files GdkPixbuf can't decode (corrupt/unsupported) rather than showing a
/// broken-image placeholder for every miss.
fn thumbnail(path: &Path, on_pick: impl Fn(PathBuf) + 'static) -> Option<Button> {
    let pixbuf = gtk4::gdk_pixbuf::Pixbuf::from_file_at_scale(path, 160, 100, true).ok()?;
    let texture = gtk4::gdk::Texture::for_pixbuf(&pixbuf);

    let card = GBox::new(Orientation::Vertical, 4);
    card.set_margin_top(4);
    card.set_margin_bottom(4);
    card.set_margin_start(4);
    card.set_margin_end(4);

    let picture = Picture::for_paintable(&texture);
    picture.set_size_request(160, 100);
    picture.set_content_fit(gtk4::ContentFit::Cover);
    card.append(&picture);

    let name = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let name_lbl = Label::new(Some(&name));
    name_lbl.add_css_class("caption");
    name_lbl.add_css_class("dim-label");
    name_lbl.set_max_width_chars(20);
    name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    card.append(&name_lbl);

    let btn = Button::new();
    btn.set_child(Some(&card));
    btn.set_tooltip_text(Some(&path.display().to_string()));
    let path = path.to_path_buf();
    btn.connect_clicked(move |_| on_pick(path.clone()));
    Some(btn)
}

fn current_wallpaper() -> Option<PathBuf> {
    let out = Command::new("breadpaper").arg("get").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

fn refresh_preview(preview: &Image, path_lbl: &Label) {
    match current_wallpaper() {
        Some(path) => {
            let filename = path.file_name().map(|f| f.to_string_lossy().to_string());
            path_lbl.set_text(filename.as_deref().unwrap_or("(unknown filename)"));
            path_lbl.set_tooltip_text(Some(&path.display().to_string()));
            preview.set_from_file(Some(&path));
        }
        None => {
            path_lbl.set_text("No wallpaper set");
            path_lbl.set_tooltip_text(None);
            preview.set_icon_name(Some("image-missing"));
        }
    }
}

/// Runs `breadpaper set <path>`, which also drives `awww img` + pywal
/// palette generation (routinely 1-3s — pywal spawns Python + an
/// ImageMagick backend, not the "sub-second" call this used to assume). GTK
/// widgets aren't `Send`, so the command runs on its own thread and the
/// result comes back over a channel (same pattern as snapshots.rs).
fn set_wallpaper(path: PathBuf, preview: Image, path_lbl: Label, status: Label) {
    status.set_text("Setting...");
    let (tx, rx) = async_channel::bounded::<bool>(1);
    std::thread::spawn(move || {
        let ok = Command::new("breadpaper").arg("set").arg(&path).status().map(|s| s.success()).unwrap_or(false);
        let _ = tx.send_blocking(ok);
    });

    glib::spawn_future_local(async move {
        let ok = rx.recv().await.unwrap_or(false);
        if ok {
            refresh_preview(&preview, &path_lbl);
            status.set_text("Wallpaper set");
        } else {
            status.set_text("breadpaper failed — see terminal/journal");
        }
        let lbl = status.clone();
        glib::timeout_add_seconds_local(3, move || {
            lbl.set_text("");
            glib::ControlFlow::Break
        });
    });
}

pub fn build() -> GBox {
    let (outer, c) = w::view_scaffold("Wallpaper");

    c.append(&w::hint(
        "Sets the desktop wallpaper, generates a matching pywal palette, and \
         reloads the shared bread-theme stylesheet — the wallpaper drives \
         the whole desktop's accent colors.",
    ));

    let preview_card = GBox::new(Orientation::Vertical, 8);
    preview_card.add_css_class("card");
    preview_card.set_halign(gtk4::Align::Center);
    preview_card.set_margin_top(8);
    preview_card.set_margin_bottom(8);

    let preview = Image::new();
    preview.set_pixel_size(320);
    preview_card.append(&preview);

    let path_lbl = Label::new(None);
    path_lbl.set_wrap(true);
    path_lbl.add_css_class("dim-label");
    preview_card.append(&path_lbl);
    c.append(&preview_card);

    refresh_preview(&preview, &path_lbl);

    let btn_row = GBox::new(Orientation::Horizontal, 8);
    btn_row.set_margin_top(8);
    btn_row.set_halign(gtk4::Align::Center);

    let choose_btn = Button::with_label("Choose image...");
    choose_btn.add_css_class("suggested-action");
    let status = Label::new(None);
    status.add_css_class("dim-label");

    {
        let preview = preview.clone();
        let path_lbl = path_lbl.clone();
        let status = status.clone();
        choose_btn.connect_clicked(move |btn| {
            let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
            let dialog = FileDialog::new();
            dialog.set_title("Choose a wallpaper");

            // Restricted to what breadpaper's own validate() actually
            // accepts — GdkPixbuf (and thus the file dialog's own preview)
            // would happily offer svg/tiff/etc. that breadpaper rejects
            // outright.
            let filter = FileFilter::new();
            for ext in WALLPAPER_EXTS {
                filter.add_suffix(ext);
            }
            filter.set_name(Some("Images"));
            let filters = gtk4::gio::ListStore::new::<FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));
            dialog.set_default_filter(Some(&filter));

            let preview = preview.clone();
            let path_lbl = path_lbl.clone();
            let status = status.clone();
            dialog.open(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        set_wallpaper(path, preview.clone(), path_lbl.clone(), status.clone());
                    }
                }
            });
        });
    }

    btn_row.append(&choose_btn);
    btn_row.append(&status);
    c.append(&btn_row);

    // Library: a thumbnail grid so picking a wallpaper doesn't mean guessing
    // blind from filenames in a file-picker list. Scanned lazily behind a
    // button click, not at panel-build time — every view is built eagerly at
    // app launch (see window.rs), and decoding dozens of scaled thumbnails
    // synchronously would add real latency to every bos-settings launch, not
    // just the first visit to this panel.
    c.append(&w::section("Library"));
    let library_dir = wallpaper_library_dir();
    let flow = FlowBox::new();
    flow.set_selection_mode(gtk4::SelectionMode::None);
    flow.set_max_children_per_line(6);
    flow.set_row_spacing(4);
    flow.set_column_spacing(4);
    flow.set_homogeneous(true);

    let flow_wrapper = GBox::new(Orientation::Vertical, 4);
    let browse_btn = Button::with_label(&format!("Browse {}", library_dir.display()));
    browse_btn.set_halign(gtk4::Align::Start);
    flow_wrapper.append(&browse_btn);
    c.append(&flow_wrapper);

    {
        let preview = preview.clone();
        let path_lbl = path_lbl.clone();
        let status = status.clone();
        let flow = flow.clone();
        let flow_wrapper = flow_wrapper.clone();
        let library_dir = library_dir.clone();
        browse_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            while let Some(child) = flow.first_child() {
                flow.remove(&child);
            }
            let paths = scan_wallpapers(&library_dir);
            if paths.is_empty() {
                flow_wrapper.append(&w::empty_state(
                    "image-missing",
                    "No wallpapers found",
                    &format!("Nothing under {} — use Choose image... above instead.", library_dir.display()),
                ));
            } else {
                for path in &paths {
                    let preview = preview.clone();
                    let path_lbl = path_lbl.clone();
                    let status = status.clone();
                    if let Some(thumb) = thumbnail(path, move |p| {
                        set_wallpaper(p, preview.clone(), path_lbl.clone(), status.clone());
                    }) {
                        flow.insert(&thumb, -1);
                    }
                }
                flow_wrapper.append(&flow);
            }
            btn.set_visible(false);
        });
    }

    outer
}
