//! Display: live-connected-monitor readout (from `hyprctl monitors -j`) plus
//! a real editor for `hypr/monitors.json` — the monitor *layout* Hyprland
//! itself reads at login via `scripts/display/monitors.lua`. Like
//! appearance.rs/autostart.rs, this is a plain typed struct round-tripped
//! whole (JSON has no comments to preserve), not the TOML `Doc` pattern.
//!
//! `Default` here (the single wildcard rule) must stay in sync with
//! `scripts/display/monitors.lua`'s own `DEFAULT_MONITORS` fallback.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation};
use serde::{Deserialize, Serialize};

use crate::ui::widgets as w;

#[derive(Clone, Serialize, Deserialize)]
struct MonitorRule {
    output: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_position")]
    position: String,
    #[serde(default = "default_scale")]
    scale: String,
}

fn default_mode() -> String {
    "preferred".to_string()
}
fn default_position() -> String {
    "auto".to_string()
}
fn default_scale() -> String {
    "auto".to_string()
}

impl Default for MonitorRule {
    fn default() -> Self {
        Self { output: String::new(), mode: default_mode(), position: default_position(), scale: default_scale() }
    }
}

#[derive(Serialize, Deserialize)]
struct MonitorsFile {
    #[serde(default)]
    monitors: Vec<MonitorRule>,
}

fn config_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/monitors.json")
}

fn load() -> Vec<MonitorRule> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str::<MonitorsFile>(&s).ok())
        .filter(|f| !f.monitors.is_empty())
        .map(|f| f.monitors)
        .unwrap_or_else(|| vec![MonitorRule::default()])
}

fn save(rules: &[MonitorRule]) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = MonitorsFile { monitors: rules.to_vec() };
    std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default())
}

fn get_live_monitors() -> Vec<(String, String)> {
    let Ok(output) = std::process::Command::new("hyprctl").args(["monitors", "-j"]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Ok(monitors) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
        return Vec::new();
    };
    monitors
        .iter()
        .filter_map(|m| {
            let name = m.get("name")?.as_str()?;
            let w = m.get("width")?.as_u64()?;
            let h = m.get("height")?.as_u64()?;
            let refresh = m.get("refreshRate")?.as_f64()?;
            Some((name.to_string(), format!("{w}x{h} @ {refresh:.0}Hz")))
        })
        .collect()
}

fn hypr_path(name: &str) -> std::path::PathBuf {
    crate::config::config_dir().join("hypr").join(name)
}

/// Open `path` in $EDITOR (nano if unset) inside a terminal window. Spawning
/// an editor directly (no terminal) is a silent no-op for any TUI editor —
/// there's nothing for it to attach to — so it always needs a terminal
/// wrapper. Uses kitty, which is what BOS actually ships (not foot).
fn open_in_terminal(path: &std::path::Path) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    if let Ok(mut child) = std::process::Command::new("kitty").args(["-e", &editor]).arg(path).spawn() {
        std::thread::spawn(move || { let _ = child.wait(); });
    }
}

fn rebuild(list: &ListBox, model: &Rc<RefCell<Vec<MonitorRule>>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, rule) in model.borrow().iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let output = Entry::new();
        output.set_text(&rule.output);
        output.set_placeholder_text(Some("any (blank = all)"));
        output.set_width_chars(12);

        let mode = Entry::new();
        mode.set_text(&rule.mode);
        mode.set_placeholder_text(Some("preferred / 1920x1080@60"));
        mode.set_width_chars(18);

        let position = Entry::new();
        position.set_text(&rule.position);
        position.set_placeholder_text(Some("auto / 0x0"));
        position.set_width_chars(10);

        let scale = Entry::new();
        scale.set_text(&rule.scale);
        scale.set_placeholder_text(Some("auto / 1"));
        scale.set_width_chars(8);
        scale.set_hexpand(true);

        let remove = Button::with_label("Remove");
        remove.add_css_class("destructive-action");

        {
            let model = model.clone();
            output.connect_changed(move |e| {
                if let Some(r) = model.borrow_mut().get_mut(i) {
                    r.output = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            mode.connect_changed(move |e| {
                if let Some(r) = model.borrow_mut().get_mut(i) {
                    r.mode = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            position.connect_changed(move |e| {
                if let Some(r) = model.borrow_mut().get_mut(i) {
                    r.position = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            scale.connect_changed(move |e| {
                if let Some(r) = model.borrow_mut().get_mut(i) {
                    r.scale = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            let list = list.clone();
            remove.connect_clicked(move |_| {
                model.borrow_mut().remove(i);
                if model.borrow().is_empty() {
                    model.borrow_mut().push(MonitorRule::default());
                }
                rebuild(&list, &model);
            });
        }

        hbox.append(&Label::new(Some("Output")));
        hbox.append(&output);
        hbox.append(&Label::new(Some("Mode")));
        hbox.append(&mode);
        hbox.append(&Label::new(Some("Position")));
        hbox.append(&position);
        hbox.append(&Label::new(Some("Scale")));
        hbox.append(&scale);
        hbox.append(&remove);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Display");

    content.append(&w::section("Connected monitors"));
    let monitors = get_live_monitors();
    if monitors.is_empty() {
        content.append(&w::hint("No monitors detected (is Hyprland running?)"));
    } else {
        for (name, mode) in &monitors {
            content.append(&w::info_row(name, mode));
        }
    }

    content.append(&w::section("Layout"));
    content.append(&w::hint(
        "One row per monitor rule. Leave Output blank to match any monitor \
         (the default — works on any hardware). Applies on next login/reload.",
    ));

    let model = Rc::new(RefCell::new(load()));
    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    rebuild(&list, &model);
    content.append(&list);

    let add_btn = Button::with_label("Add monitor rule");
    add_btn.set_halign(gtk4::Align::Start);
    add_btn.set_margin_top(6);
    {
        let model = model.clone();
        let list = list.clone();
        add_btn.connect_clicked(move |_| {
            model.borrow_mut().push(MonitorRule::default());
            rebuild(&list, &model);
        });
    }
    content.append(&add_btn);

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);
    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let status = Label::new(None);
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

    content.append(&w::section("Advanced"));
    let open_btn = Button::with_label("Open hyprland.lua in editor");
    open_btn.set_halign(gtk4::Align::Start);
    {
        let conf_path = hypr_path("hyprland.lua");
        open_btn.connect_clicked(move |_| open_in_terminal(&conf_path));
    }
    content.append(&open_btn);

    // breadhelp is the real keybind viewer now (SUPER+/) — /usr/share/bos/
    // keybinds.txt was deleted when breadhelp replaced bos-keybinds/
    // bos-welcome, so this used to open a file that no longer exists.
    let keybinds_btn = Button::with_label("View keybinds (breadhelp)");
    keybinds_btn.set_halign(gtk4::Align::Start);
    keybinds_btn.connect_clicked(move |_| {
        let _ = std::process::Command::new("breadhelp").spawn();
    });
    content.append(&keybinds_btn);

    outer
}
