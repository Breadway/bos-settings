//! hypr/binds.json — Hyprland keybind editor, read by
//! `scripts/ui/binds.lua` on the Hyprland side (see hyprland.lua). The
//! schema has four kinds of bind lists (`globals`, `common`, and one per
//! keyboard `layouts` entry) and each bind's shape varies by `action`
//! (`exec` needs `command`, `move_dir` needs `direction`, workspace-focus
//! needs `workspace`, mouse binds need `options.mouse`, ...). Rather than
//! modelling every action's field set as its own row layout — which would
//! mean a combinatorial explosion of widgets and silently dropping any
//! action shape this editor doesn't already know about — `action`/`key`/
//! `mods` get real fields (the ones every bind has) and everything else
//! round-trips through `#[serde(flatten)]` into a small inline-JSON column,
//! same trade-off the other Hyprland JSON editors (appearance.rs,
//! hyprland.rs, autostart.rs) already make: no comments to preserve, so this
//! is a whole-file round trip, not the `toml_edit`/`Doc` path-based pattern.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    AlertDialog, Box as GBox, Button, DropDown, Entry, Expression, Label, ListBox, ListBoxRow,
    Orientation, StringList,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ui::widgets as w;

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct Bind {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mods: Vec<String>,
    /// Everything else a bind can carry — `command`, `direction`,
    /// `workspace`, `x`, `y`, `layout`, `options`, and any action shape not
    /// yet invented. Edited as compact inline JSON (see `extra_field`).
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct BindsFile {
    active_layout: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    default_mods: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    globals: Vec<Bind>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    common: Vec<Bind>,
    /// A `BTreeMap` (alphabetical), not the file's original insertion order
    /// — same "whole-file round trip, formatting not preserved" trade-off as
    /// the rest of this file's JSON-config siblings.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    layouts: BTreeMap<String, Vec<Bind>>,
}

fn config_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/binds.json")
}

fn load() -> BindsFile {
    std::fs::read_to_string(config_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save(f: &BindsFile) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(f).unwrap_or_default())
}

fn mods_to_text(mods: &[String]) -> String {
    mods.join(", ")
}

fn text_to_mods(s: &str) -> Vec<String> {
    s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

/// A `Vec<Bind>` accessor that always finds the right list regardless of
/// whether it's `globals`, `common`, or a named entry in `layouts` — lets one
/// row-builder work for every section instead of three near-duplicates.
type SectionAccessor = Rc<dyn Fn(&mut BindsFile) -> &mut Vec<Bind>>;

fn globals_accessor() -> SectionAccessor {
    Rc::new(|f| &mut f.globals)
}
fn common_accessor() -> SectionAccessor {
    Rc::new(|f| &mut f.common)
}
fn layout_accessor(name: String) -> SectionAccessor {
    Rc::new(move |f| f.layouts.entry(name.clone()).or_default())
}

fn bind_row(
    model: &Rc<RefCell<BindsFile>>,
    accessor: &SectionAccessor,
    idx: usize,
    rerender: &Rc<dyn Fn()>,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    let hbox = GBox::new(Orientation::Horizontal, 6);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);
    hbox.set_margin_start(6);
    hbox.set_margin_end(6);

    let (mods_cur, key_cur, action_cur, extra_cur) = {
        let mut m = model.borrow_mut();
        let bind = &accessor(&mut m)[idx];
        (
            mods_to_text(&bind.mods),
            bind.key.clone().unwrap_or_default(),
            bind.action.clone(),
            if bind.extra.is_empty() { String::new() } else { serde_json::to_string(&bind.extra).unwrap_or_default() },
        )
    };

    let mods = Entry::new();
    mods.set_text(&mods_cur);
    mods.set_placeholder_text(Some("SUPER, SHIFT"));
    mods.set_width_chars(14);

    let key = Entry::new();
    key.set_text(&key_cur);
    key.set_placeholder_text(Some("key"));
    key.set_width_chars(8);

    let action = Entry::new();
    action.set_text(&action_cur);
    action.set_placeholder_text(Some("exec / focus / move_dir / ..."));
    action.set_width_chars(12);

    let extra = Entry::new();
    extra.set_text(&extra_cur);
    extra.set_placeholder_text(Some(r#"{"command": "..."}"#));
    extra.set_hexpand(true);

    let remove = Button::with_label("Remove");
    remove.add_css_class("destructive-action");

    {
        let model = model.clone();
        let accessor = accessor.clone();
        mods.connect_changed(move |e| {
            let mut m = model.borrow_mut();
            if let Some(b) = accessor(&mut m).get_mut(idx) {
                b.mods = text_to_mods(&e.text());
            }
        });
    }
    {
        let model = model.clone();
        let accessor = accessor.clone();
        key.connect_changed(move |e| {
            let mut m = model.borrow_mut();
            if let Some(b) = accessor(&mut m).get_mut(idx) {
                let t = e.text().to_string();
                b.key = if t.is_empty() { None } else { Some(t) };
            }
        });
    }
    {
        let model = model.clone();
        let accessor = accessor.clone();
        action.connect_changed(move |e| {
            let mut m = model.borrow_mut();
            if let Some(b) = accessor(&mut m).get_mut(idx) {
                b.action = e.text().to_string();
            }
        });
    }
    {
        let model = model.clone();
        let accessor = accessor.clone();
        // Only commit on valid JSON — otherwise every keystroke while
        // composing e.g. `{"command":"kitty"}` would momentarily wipe the
        // bind's extra fields the instant the text isn't parseable yet.
        extra.connect_changed(move |e| {
            let text = e.text();
            let parsed: Result<Map<String, Value>, _> =
                if text.trim().is_empty() { Ok(Map::new()) } else { serde_json::from_str(&text) };
            match parsed {
                Ok(map) => {
                    e.remove_css_class("error");
                    let mut m = model.borrow_mut();
                    if let Some(b) = accessor(&mut m).get_mut(idx) {
                        b.extra = map;
                    }
                }
                Err(_) => e.add_css_class("error"),
            }
        });
    }
    {
        let model = model.clone();
        let accessor = accessor.clone();
        let rerender = rerender.clone();
        remove.connect_clicked(move |_| {
            accessor(&mut model.borrow_mut()).remove(idx);
            rerender();
        });
    }

    hbox.append(&mods);
    hbox.append(&key);
    hbox.append(&action);
    hbox.append(&extra);
    hbox.append(&remove);
    row.set_child(Some(&hbox));
    row
}

/// A section = a title, an "Add bind" button, and the section's bind rows —
/// shared by Globals, Common, and every named layout.
fn section(
    title: Option<&str>,
    model: &Rc<RefCell<BindsFile>>,
    accessor: SectionAccessor,
    rerender: &Rc<dyn Fn()>,
) -> GBox {
    let wrapper = GBox::new(Orientation::Vertical, 4);
    if let Some(title) = title {
        wrapper.append(&w::section(title));
    }

    let list = ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::None);
    list.add_css_class("boxed-list");
    let count = accessor(&mut model.borrow_mut()).len();
    for i in 0..count {
        list.append(&bind_row(model, &accessor, i, rerender));
    }
    wrapper.append(&list);

    let add_btn = Button::with_label("Add bind");
    add_btn.set_halign(gtk4::Align::Start);
    add_btn.set_margin_top(4);
    {
        let model = model.clone();
        let accessor = accessor.clone();
        let rerender = rerender.clone();
        add_btn.connect_clicked(move |_| {
            accessor(&mut model.borrow_mut()).push(Bind { action: "exec".to_string(), ..Default::default() });
            rerender();
        });
    }
    wrapper.append(&add_btn);
    wrapper
}

fn rerender(content: &GBox, model: &Rc<RefCell<BindsFile>>, status: &Label) {
    while let Some(child) = content.first_child() {
        content.remove(&child);
    }
    populate(content, model, status);
}

fn populate(content: &GBox, model: &Rc<RefCell<BindsFile>>, status: &Label) {
    let rerender: Rc<dyn Fn()> = {
        let content = content.clone();
        let model = model.clone();
        let status = status.clone();
        Rc::new(move || rerender(&content, &model, &status))
    };

    content.append(&w::hint(
        "Mods/Key/Action are the fields every bind needs. The last column \
         holds action-specific extras as inline JSON — e.g. \
         {\"command\": \"kitty\"}, {\"direction\": \"left\"}, \
         {\"workspace\": \"e+1\"}, {\"options\": {\"repeating\": true}} — \
         leave it blank for actions with none (close, exit, fullscreen, ...). \
         Applies on next login/reload.",
    ));

    let layout_names: Vec<String> = model.borrow().layouts.keys().cloned().collect();

    let top_row = GBox::new(Orientation::Horizontal, 12);
    top_row.append(&{
        let dd = DropDown::new(
            Some(StringList::new(&layout_names.iter().map(String::as_str).collect::<Vec<_>>())),
            Expression::NONE,
        );
        let cur = model.borrow().active_layout.clone();
        dd.set_selected(layout_names.iter().position(|n| *n == cur).unwrap_or(0) as u32);
        let model = model.clone();
        let layout_names = layout_names.clone();
        dd.connect_selected_notify(move |dd| {
            if let Some(name) = layout_names.get(dd.selected() as usize) {
                model.borrow_mut().active_layout = name.clone();
            }
        });
        w::row("Active layout", &dd)
    });
    content.append(&top_row);

    let default_mods = Entry::new();
    default_mods.set_text(&mods_to_text(&model.borrow().default_mods));
    default_mods.set_hexpand(true);
    default_mods.set_width_chars(20);
    {
        let model = model.clone();
        default_mods.connect_changed(move |e| {
            model.borrow_mut().default_mods = text_to_mods(&e.text());
        });
    }
    content.append(&w::row("Default mods", &default_mods));

    content.append(&section(Some("Media & function keys (globals)"), model, globals_accessor(), &rerender));
    content.append(&section(Some("Common (every layout)"), model, common_accessor(), &rerender));

    for name in &layout_names {
        let header = GBox::new(Orientation::Horizontal, 8);
        let title = Label::new(Some(&format!("Layout: {name}")));
        title.add_css_class("heading");
        title.set_hexpand(true);
        title.set_xalign(0.0);
        title.set_margin_top(12);
        header.append(&title);
        let remove_layout = Button::with_label("Remove layout");
        remove_layout.add_css_class("destructive-action");
        {
            let model = model.clone();
            let rerender = rerender.clone();
            let name = name.clone();
            remove_layout.connect_clicked(move |btn| {
                let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                let dialog = AlertDialog::builder()
                    .message(format!("Remove layout \"{name}\"?"))
                    .detail("Deletes every bind defined under this layout. This can't be undone here.")
                    .buttons(["Cancel", "Remove"])
                    .cancel_button(0)
                    .default_button(0)
                    .build();
                let model = model.clone();
                let rerender = rerender.clone();
                let name = name.clone();
                dialog.choose(window.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                    if result == Ok(1) {
                        let mut m = model.borrow_mut();
                        m.layouts.remove(&name);
                        if m.active_layout == name {
                            m.active_layout = m.layouts.keys().next().cloned().unwrap_or_default();
                        }
                        drop(m);
                        rerender();
                    }
                });
            });
        }
        header.append(&remove_layout);
        content.append(&header);

        // No section title here — the "Layout: X" header above (with its
        // own Remove button) already covers it.
        content.append(&section(None, model, layout_accessor(name.clone()), &rerender));
    }

    let add_layout_row = GBox::new(Orientation::Horizontal, 8);
    add_layout_row.set_margin_top(12);
    let new_layout_name = Entry::new();
    new_layout_name.set_placeholder_text(Some("new layout name"));
    let add_layout_btn = Button::with_label("Add layout");
    {
        let model = model.clone();
        let rerender = rerender.clone();
        let entry = new_layout_name.clone();
        add_layout_btn.connect_clicked(move |_| {
            let name = entry.text().trim().to_string();
            if name.is_empty() {
                return;
            }
            model.borrow_mut().layouts.entry(name).or_default();
            entry.set_text("");
            rerender();
        });
    }
    add_layout_row.append(&new_layout_name);
    add_layout_row.append(&add_layout_btn);
    content.append(&add_layout_row);
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Keybinds");
    let model = Rc::new(RefCell::new(load()));

    let status = Label::new(None);
    status.add_css_class("dim-label");

    populate(&content, &model, &status);

    let btn_row = GBox::new(Orientation::Horizontal, 12);
    btn_row.set_margin_top(16);
    let save_btn = Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
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
