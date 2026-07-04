//! hypr/settings.json — Hyprland gaps/borders/blur/shadow/input, read by
//! `scripts/ui/settings.lua` on the Hyprland side (see hyprland.lua). Unlike
//! the TOML-backed views, this has no comments to preserve, so it's modeled
//! as a plain typed struct (round-tripped whole) rather than the
//! `toml_edit`/`Doc` path-based editor the other panels use.
//!
//! `Default` here must stay in sync with `scripts/ui/settings.lua`'s
//! `DEFAULTS` table on the Hyprland side — both independently define "what
//! BOS ships out of the box," the same duplication `binds.json`/
//! `content/keybinds.rs` already accept for the same reason (two different
//! languages/processes reading the same file, neither able to import the
//! other's defaults).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GBox, ColorDialog, ColorDialogButton, DropDown, Entry, Expression, Orientation,
    SpinButton, StringList, Switch,
};
use serde::{Deserialize, Serialize};

use crate::ui::widgets as w;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Appearance {
    gaps_in: i64,
    gaps_out: i64,
    border_size: i64,
    active_border: String,
    inactive_border: String,
    layout: String,
    resize_on_border: bool,
    rounding: i64,
    blur_enabled: bool,
    blur_size: i64,
    blur_passes: i64,
    shadow_enabled: bool,
    shadow_range: i64,
    shadow_render_power: i64,
    kb_layout: String,
    follow_mouse: i64,
    natural_scroll: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            gaps_in: 5,
            gaps_out: 10,
            border_size: 2,
            active_border: "rgba(88c0d0ff)".to_string(),
            inactive_border: "rgba(4c566aff)".to_string(),
            layout: "dwindle".to_string(),
            resize_on_border: true,
            rounding: 8,
            blur_enabled: true,
            blur_size: 6,
            blur_passes: 2,
            shadow_enabled: true,
            shadow_range: 12,
            shadow_render_power: 3,
            kb_layout: "us".to_string(),
            follow_mouse: 1,
            natural_scroll: true,
        }
    }
}

fn config_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/settings.json")
}

/// A missing or malformed file yields defaults — same failsafe posture as
/// the Lua loader reading this same file on the Hyprland side.
fn load() -> Appearance {
    std::fs::read_to_string(config_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save(a: &Appearance) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(a).unwrap_or_default())
}

/// "rgba(RRGGBBAA)" (Hyprland's format) <-> gdk::RGBA, so the color fields
/// get a real color-picker button instead of a raw hex text field.
fn parse_hypr_color(s: &str) -> Option<gtk4::gdk::RGBA> {
    let inner = s.strip_prefix("rgba(")?.strip_suffix(')')?;
    if inner.len() != 8 {
        return None;
    }
    let r = u8::from_str_radix(&inner[0..2], 16).ok()?;
    let g = u8::from_str_radix(&inner[2..4], 16).ok()?;
    let b = u8::from_str_radix(&inner[4..6], 16).ok()?;
    let a = u8::from_str_radix(&inner[6..8], 16).ok()?;
    Some(gtk4::gdk::RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0))
}

fn to_hypr_color(c: &gtk4::gdk::RGBA) -> String {
    let clamp = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("rgba({:02x}{:02x}{:02x}{:02x})", clamp(c.red()), clamp(c.green()), clamp(c.blue()), clamp(c.alpha()))
}

fn color_row(label: &str, model: &Rc<RefCell<Appearance>>, get: fn(&Appearance) -> &str, set: fn(&mut Appearance, String)) -> GBox {
    let cur = parse_hypr_color(get(&model.borrow())).unwrap_or(gtk4::gdk::RGBA::new(0.5, 0.5, 0.5, 1.0));
    let btn = ColorDialogButton::new(Some(ColorDialog::builder().with_alpha(true).build()));
    btn.set_rgba(&cur);
    let model = model.clone();
    btn.connect_rgba_notify(move |b| {
        set(&mut model.borrow_mut(), to_hypr_color(&b.rgba()));
    });
    w::row(label, &btn)
}

fn spin_row(label: &str, model: &Rc<RefCell<Appearance>>, min: f64, max: f64, get: fn(&Appearance) -> i64, set: fn(&mut Appearance, i64)) -> GBox {
    let cur = get(&model.borrow());
    let adj = Adjustment::new(cur as f64, min, max, 1.0, 1.0, 0.0);
    let spin = SpinButton::new(Some(&adj), 1.0, 0);
    let model = model.clone();
    spin.connect_value_changed(move |s| set(&mut model.borrow_mut(), s.value() as i64));
    w::row(label, &spin)
}

fn switch_row(label: &str, model: &Rc<RefCell<Appearance>>, get: fn(&Appearance) -> bool, set: fn(&mut Appearance, bool)) -> GBox {
    let sw = Switch::new();
    sw.set_active(get(&model.borrow()));
    let model = model.clone();
    sw.connect_active_notify(move |s| set(&mut model.borrow_mut(), s.is_active()));
    w::row(label, &sw)
}

fn entry_row(label: &str, model: &Rc<RefCell<Appearance>>, get: fn(&Appearance) -> String, set: fn(&mut Appearance, String)) -> GBox {
    let entry = Entry::new();
    entry.set_text(&get(&model.borrow()));
    entry.set_hexpand(true);
    entry.set_width_chars(16);
    let model = model.clone();
    entry.connect_changed(move |e| set(&mut model.borrow_mut(), e.text().to_string()));
    w::row(label, &entry)
}

fn dropdown_row(label: &str, model: &Rc<RefCell<Appearance>>, options: &[&str], get: fn(&Appearance) -> &str, set: fn(&mut Appearance, String)) -> GBox {
    let cur = get(&model.borrow()).to_string();
    let dd = DropDown::new(Some(StringList::new(options)), Expression::NONE);
    dd.set_selected(options.iter().position(|o| *o == cur).unwrap_or(0) as u32);
    let owned: Vec<String> = options.iter().map(|s| s.to_string()).collect();
    let model = model.clone();
    dd.connect_selected_notify(move |dd| {
        if let Some(opt) = owned.get(dd.selected() as usize) {
            set(&mut model.borrow_mut(), opt.clone());
        }
    });
    w::row(label, &dd)
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Appearance");

    content.append(&w::hint(
        "Gaps, borders, blur, and input feel for Hyprland — the same \
         settings.json the compositor itself reads at login.",
    ));

    let model = Rc::new(RefCell::new(load()));

    content.append(&w::section("Layout & borders"));
    content.append(&spin_row("Gaps (inner)", &model, 0.0, 50.0, |a| a.gaps_in, |a, v| a.gaps_in = v));
    content.append(&spin_row("Gaps (outer)", &model, 0.0, 50.0, |a| a.gaps_out, |a, v| a.gaps_out = v));
    content.append(&spin_row("Border width", &model, 0.0, 10.0, |a| a.border_size, |a, v| a.border_size = v));
    content.append(&color_row("Active border color", &model, |a| &a.active_border, |a, v| a.active_border = v));
    content.append(&color_row("Inactive border color", &model, |a| &a.inactive_border, |a, v| a.inactive_border = v));
    content.append(&dropdown_row("Tiling layout", &model, &["dwindle", "master"], |a| &a.layout, |a, v| a.layout = v));
    content.append(&switch_row("Resize by dragging borders", &model, |a| a.resize_on_border, |a, v| a.resize_on_border = v));

    content.append(&w::section("Effects"));
    content.append(&spin_row("Corner rounding", &model, 0.0, 30.0, |a| a.rounding, |a, v| a.rounding = v));
    content.append(&switch_row("Blur", &model, |a| a.blur_enabled, |a, v| a.blur_enabled = v));
    content.append(&spin_row("Blur size", &model, 0.0, 20.0, |a| a.blur_size, |a, v| a.blur_size = v));
    content.append(&spin_row("Blur passes", &model, 1.0, 5.0, |a| a.blur_passes, |a, v| a.blur_passes = v));
    content.append(&switch_row("Window shadows", &model, |a| a.shadow_enabled, |a, v| a.shadow_enabled = v));
    content.append(&spin_row("Shadow range", &model, 0.0, 40.0, |a| a.shadow_range, |a, v| a.shadow_range = v));
    content.append(&spin_row("Shadow render power", &model, 1.0, 4.0, |a| a.shadow_render_power, |a, v| a.shadow_render_power = v));

    content.append(&w::section("Input"));
    content.append(&entry_row("Keyboard layout", &model, |a| a.kb_layout.clone(), |a, v| a.kb_layout = v));
    let follow_mouse_row = spin_row("Focus-follows-mouse mode", &model, 0.0, 3.0, |a| a.follow_mouse, |a, v| a.follow_mouse = v);
    content.append(&follow_mouse_row);
    content.append(&w::hint("0-3 — see the Hyprland wiki's follow_mouse setting for exact behavior of each value."));
    content.append(&switch_row("Natural scrolling (touchpad)", &model, |a| a.natural_scroll, |a, v| a.natural_scroll = v));

    content.append(&w::hint("Applies on next login/Hyprland reload — this saves settings.json, it doesn't reload Hyprland live."));

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);
    let save_btn = gtk4::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status = gtk4::Label::new(None);
    status.add_css_class("dim-label");
    {
        let model = model.clone();
        let status = status.clone();
        save_btn.connect_clicked(move |_| match save(&model.borrow()) {
            Ok(()) => {
                status.set_text("Saved");
                let lbl = status.clone();
                glib::timeout_add_seconds_local(3, move || {
                    lbl.set_text("");
                    glib::ControlFlow::Break
                });
            }
            Err(e) => status.set_text(&format!("Error: {e}")),
        });
    }
    btn_row.append(&save_btn);
    btn_row.append(&status);
    outer.append(&btn_row);

    outer
}
