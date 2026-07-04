//! hypr/autostart.json — the *extra*, user-toggleable autostart apps
//! (breadbar, hypridle, bos-netcheck, breadhelp, plus anything a user adds).
//! The core bootstrap sequence (theme generation, dark-mode gsettings,
//! polkit agent, wallpaper daemon, breadd's Wayland-env fix, breadclipd)
//! stays hardcoded in hyprland.lua on purpose — it's timing/order-sensitive
//! infrastructure, not something this panel exposes.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, Switch};
use serde::{Deserialize, Serialize};

use crate::ui::widgets as w;

#[derive(Clone, Serialize, Deserialize)]
struct Entry_ {
    command: String,
    #[serde(default)]
    label: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
struct AutostartFile {
    #[serde(default)]
    extra: Vec<Entry_>,
}

fn default_extra() -> Vec<Entry_> {
    vec![
        Entry_ { command: "breadbar".to_string(), label: "Bar (breadbar)".to_string(), enabled: true },
        Entry_ { command: "hypridle".to_string(), label: "Idle / lock daemon (hypridle)".to_string(), enabled: true },
        Entry_ { command: "bos-netcheck".to_string(), label: "Network connectivity check".to_string(), enabled: true },
        Entry_ { command: "breadhelp --autostart".to_string(), label: "BOS Help (first-run onboarding)".to_string(), enabled: true },
    ]
}

fn config_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/autostart.json")
}

fn load() -> Vec<Entry_> {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str::<AutostartFile>(&s).ok())
        .map(|f| f.extra)
        .unwrap_or_else(default_extra)
}

fn save(entries: &[Entry_]) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = AutostartFile { extra: entries.to_vec() };
    std::fs::write(path, serde_json::to_string_pretty(&file).unwrap_or_default())
}

fn rebuild(list: &ListBox, model: &Rc<RefCell<Vec<Entry_>>>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    for (i, entry) in model.borrow().iter().enumerate() {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        let hbox = GBox::new(Orientation::Horizontal, 8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);

        let enabled = Switch::new();
        enabled.set_active(entry.enabled);
        enabled.set_valign(gtk4::Align::Center);

        let label = Entry::new();
        label.set_text(&entry.label);
        label.set_placeholder_text(Some("Label"));
        label.set_width_chars(20);

        let command = Entry::new();
        command.set_text(&entry.command);
        command.set_placeholder_text(Some("command"));
        command.set_hexpand(true);

        let remove = Button::with_label("Remove");
        remove.add_css_class("destructive-action");

        {
            let model = model.clone();
            enabled.connect_active_notify(move |s| {
                if let Some(e) = model.borrow_mut().get_mut(i) {
                    e.enabled = s.is_active();
                }
            });
        }
        {
            let model = model.clone();
            label.connect_changed(move |e| {
                if let Some(entry) = model.borrow_mut().get_mut(i) {
                    entry.label = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            command.connect_changed(move |e| {
                if let Some(entry) = model.borrow_mut().get_mut(i) {
                    entry.command = e.text().to_string();
                }
            });
        }
        {
            let model = model.clone();
            let list = list.clone();
            remove.connect_clicked(move |_| {
                model.borrow_mut().remove(i);
                rebuild(&list, &model);
            });
        }

        hbox.append(&enabled);
        hbox.append(&label);
        hbox.append(&command);
        hbox.append(&remove);
        row.set_child(Some(&hbox));
        list.append(&row);
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Startup Apps");

    content.append(&w::hint(
        "What launches after login, beyond the core desktop (bar, theme, \
         clipboard, etc. always start regardless). Toggle off, edit, or add \
         your own — each row is one command run at login.",
    ));

    let model = Rc::new(RefCell::new(load()));

    content.append(&w::section("Extra autostart apps"));
    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    rebuild(&list, &model);
    content.append(&list);

    let add_btn = Button::with_label("Add app");
    add_btn.set_halign(gtk4::Align::Start);
    add_btn.set_margin_top(6);
    {
        let model = model.clone();
        let list = list.clone();
        add_btn.connect_clicked(move |_| {
            model.borrow_mut().push(Entry_ { command: String::new(), label: String::new(), enabled: true });
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
        save_btn.connect_clicked(move |_| {
            // Empty command rows (still-being-typed "Add app" entries) are
            // dropped on save rather than written as a broken autostart.json
            // entry the Lua loader would otherwise have to reject.
            let entries: Vec<Entry_> = model.borrow().iter().filter(|e| !e.command.trim().is_empty()).cloned().collect();
            match save(&entries) {
                Ok(()) => {
                    status.set_text("Saved");
                    let lbl = status.clone();
                    glib::timeout_add_seconds_local(3, move || {
                        lbl.set_text("");
                        glib::ControlFlow::Break
                    });
                }
                Err(e) => status.set_text(&format!("Error: {e}")),
            }
        });
    }
    btn_row.append(&save_btn);
    btn_row.append(&status);
    outer.append(&btn_row);

    outer
}
