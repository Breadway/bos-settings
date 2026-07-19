//! hypr/binds.json — Hyprland keybind editor, read by
//! `scripts/ui/binds.lua` on the Hyprland side (see hyprland.lua).
//!
//! This file has TWO real on-disk shapes, and which one applies depends on
//! the machine:
//!
//! - **Flat** (`default_mods` + a single `bindings` array) — what BOS itself
//!   ships (`iso/airootfs/etc/skel/.config/hypr/binds.json`, read by the
//!   BOS-shipped `scripts/input/binds.lua`). No layouts. Each bind carries
//!   `label`/`category`/`demo_cmd` fields breadhelp depends on for its
//!   cheatsheet and guided tour.
//! - **MultiLayout** (`globals`/`common`/one `layouts` entry per keyboard
//!   layout) — a personal, per-machine schema some dev setups use instead,
//!   read by a different, personal `binds.lua`.
//!
//! This editor was originally built only against the MultiLayout shape.
//! Loading a real BOS (Flat) file into that model, then saving, silently
//! dropped the `bindings` key entirely — still valid JSON, so the Lua
//! `pcall` failsafes on the reading side never caught it. `SchemaKind`
//! detects which shape is actually on disk (from the top-level key set) and
//! `save()` always emits that SAME shape back — see `SchemaKind::detect`
//! and `save_to`.
//!
//! Each bind's shape also varies by `action` (`exec` needs `command`,
//! `move_dir` needs `direction`, workspace-focus needs `workspace`, mouse
//! binds need `options.mouse`, ...). Rather than modelling every action's
//! field set as its own row layout — which would mean a combinatorial
//! explosion of widgets and silently dropping any action shape this editor
//! doesn't already know about — `action`/`key`/`mods` get real fields (the
//! ones every bind has) and everything else round-trips through
//! `#[serde(flatten)]` into a small inline-JSON column, same trade-off the
//! other Hyprland JSON editors (appearance.rs, hyprland.rs, autostart.rs)
//! already make: no comments to preserve, so this is a whole-file round
//! trip, not the `toml_edit`/`Doc` path-based pattern.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    AlertDialog, Box as GBox, Button, DropDown, Entry, Expression, Label, ListBox, ListBoxRow,
    Orientation, StringList,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ui::widgets as w;

/// Which on-disk shape `binds.json` was loaded as. Detected once at load
/// time from the top-level key set present in the JSON, then pinned for the
/// lifetime of the editor session so `save()` always writes back the same
/// shape it read, regardless of what the in-memory model happens to have
/// populated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SchemaKind {
    /// `{ "default_mods": [...], "bindings": [...] }` — BOS's real shipped
    /// shape. No layout-switching UI applies; there's nothing to switch.
    Flat,
    /// `{ "active_layout", "default_mods", "globals", "common", "layouts" }`
    /// — the personal, multi-keyboard-layout schema this editor was
    /// originally built against.
    MultiLayout,
    /// Neither key set matched — an empty file, a totally different shape,
    /// or unparsable JSON. Loading still renders (empty), but `save()`
    /// refuses outright rather than guessing a shape and risking silently
    /// destroying whatever the real file's actual schema was.
    Unknown,
}

impl SchemaKind {
    fn detect(top_level: &Map<String, Value>) -> Self {
        if top_level.contains_key("bindings") {
            SchemaKind::Flat
        } else if top_level.contains_key("globals")
            || top_level.contains_key("common")
            || top_level.contains_key("layouts")
        {
            SchemaKind::MultiLayout
        } else {
            SchemaKind::Unknown
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
struct Bind {
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    /// `None` (key omitted) means "fall back to `default_mods`"; `Some(_)`
    /// — including `Some(vec![])` — means "use exactly this, even if that's
    /// no modifiers at all." Real BOS binds rely on that distinction (e.g.
    /// media keys pin `"mods": []` on purpose so they never inherit
    /// `default_mods`), so this can't collapse both cases to "omit the
    /// key" the way a bare `Vec<String>` with `skip_serializing_if` would —
    /// that would silently turn an explicit "no mods" into "use the
    /// default" the next time this editor saves the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    mods: Option<Vec<String>>,
    /// Everything else a bind can carry — `command`, `direction`,
    /// `workspace`, `x`, `y`, `layout`, `options`, `label`, `category`,
    /// `demo_cmd`, and any action shape not yet invented. Edited as compact
    /// inline JSON (see `extra_field`). This flatten is what keeps
    /// breadhelp's `label`/`category`/`demo_cmd` fields — which this
    /// editor's UI has no dedicated widgets for — alive across a full
    /// load/save round trip instead of being silently dropped.
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct BindsFile {
    #[serde(skip_serializing_if = "String::is_empty")]
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
    /// Flat-schema bind list — BOS's real shipped shape. Only ever populated
    /// when `SchemaKind::Flat` was detected at load time; stays empty (and
    /// so omitted, see `to_json`) for a MultiLayout file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    bindings: Vec<Bind>,
}

/// The editor's full in-memory state: which shape was loaded, plus the data
/// itself. Kept together so a stray code path can't accidentally serialize
/// `file` without knowing which shape it's supposed to come back out as.
struct Model {
    kind: SchemaKind,
    file: BindsFile,
}

fn config_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/binds.json")
}

fn load_from(path: &Path) -> (BindsFile, SchemaKind) {
    let Ok(text) = std::fs::read_to_string(path) else {
        // No file yet (fresh install/environment) — nothing on disk to
        // misdetect or destroy. BOS itself ships the flat schema, so a new
        // file defaults to Flat rather than the personal MultiLayout schema
        // this editor originally assumed.
        return (BindsFile::default(), SchemaKind::Flat);
    };
    let kind = match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(top_level)) => SchemaKind::detect(&top_level),
        // Unparsable JSON, or valid JSON that isn't even an object — treat
        // as Unknown so save() refuses rather than silently overwriting
        // whatever this file actually was with an empty default.
        _ => SchemaKind::Unknown,
    };
    let file: BindsFile = serde_json::from_str(&text).unwrap_or_default();
    (file, kind)
}

fn load() -> (BindsFile, SchemaKind) {
    load_from(&config_path())
}

/// Serialize `f` in exactly the shape `kind` implies:
/// - `Flat` -> `{ "default_mods": [...], "bindings": [...] }`, nothing else
///   — no `active_layout`/`globals`/`common`/`layouts` keys, even if the
///   struct happens to carry empty values for them.
/// - `MultiLayout` -> today's existing shape (whatever fields are
///   non-empty), via `BindsFile`'s own `Serialize` impl.
fn to_json(f: &BindsFile, kind: SchemaKind) -> Value {
    match kind {
        SchemaKind::Flat => serde_json::json!({
            "default_mods": f.default_mods,
            "bindings": f.bindings,
        }),
        SchemaKind::MultiLayout => serde_json::to_value(f).unwrap_or(Value::Null),
        SchemaKind::Unknown => Value::Null,
    }
}

fn save_to(path: &Path, f: &BindsFile, kind: SchemaKind) -> std::io::Result<()> {
    if kind == SchemaKind::Unknown {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "binds.json's schema wasn't recognized (expected a \"bindings\" key, or one of \
             \"globals\"/\"common\"/\"layouts\") — refusing to save so nothing gets silently \
             overwritten. Fix or remove the file, then reopen this panel.",
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&to_json(f, kind)).unwrap_or_default();
    crate::config::atomic_write(path, &text)
}

fn save(f: &BindsFile, kind: SchemaKind) -> std::io::Result<()> {
    save_to(&config_path(), f, kind)
}

fn mods_to_text(mods: &[String]) -> String {
    mods.join(", ")
}

fn text_to_mods(s: &str) -> Vec<String> {
    s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

/// A `Vec<Bind>` accessor that always finds the right list regardless of
/// whether it's `globals`, `common`, a named entry in `layouts`, or the flat
/// `bindings` list — lets one row-builder work for every section instead of
/// near-duplicates per schema.
type SectionAccessor = Rc<dyn Fn(&mut Model) -> &mut Vec<Bind>>;

fn globals_accessor() -> SectionAccessor {
    Rc::new(|m| &mut m.file.globals)
}
fn common_accessor() -> SectionAccessor {
    Rc::new(|m| &mut m.file.common)
}
fn layout_accessor(name: String) -> SectionAccessor {
    Rc::new(move |m| m.file.layouts.entry(name.clone()).or_default())
}
fn bindings_accessor() -> SectionAccessor {
    Rc::new(|m| &mut m.file.bindings)
}

fn bind_row(
    model: &Rc<RefCell<Model>>,
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
            mods_to_text(bind.mods.as_deref().unwrap_or(&[])),
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
                // Explicitly setting this field (even to an empty string,
                // which `text_to_mods` turns into `vec![]`) always records
                // `Some(_)` — "use exactly these mods" — never falls back
                // to inferring "key omitted" from an empty result.
                b.mods = Some(text_to_mods(&e.text()));
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
/// shared by Globals, Common, every named layout, and the flat Bindings list.
fn section(
    title: Option<&str>,
    model: &Rc<RefCell<Model>>,
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

fn rerender(content: &GBox, model: &Rc<RefCell<Model>>, status: &Label) {
    while let Some(child) = content.first_child() {
        content.remove(&child);
    }
    populate(content, model, status);
}

fn populate_unknown(content: &GBox, status: &Label) {
    content.append(&w::hint(
        "binds.json's schema wasn't recognized (expected a \"bindings\" key, or one of \
         \"globals\"/\"common\"/\"layouts\"). Nothing below is editable, and Save is disabled, \
         so the file on disk isn't at risk of being silently overwritten with the wrong shape. \
         Fix or remove the file by hand, then reopen this panel.",
    ));
    status.set_text("binds.json schema not recognized — editing disabled");
}

fn populate_flat(content: &GBox, model: &Rc<RefCell<Model>>, status: &Label, rerender: &Rc<dyn Fn()>) {
    content.append(&w::hint(
        "Mods/Key/Action are the fields every bind needs. The last column holds action-specific \
         extras as inline JSON — e.g. {\"command\": \"kitty\"}, {\"direction\": \"left\"}, \
         {\"workspace\": \"e+1\"}, {\"label\": \"...\", \"category\": \"...\"} — leave it blank \
         for actions with none (close, exit, fullscreen, ...). This machine's binds.json uses \
         BOS's flat schema (no keyboard-layout switching), so that's all there is. Applies on \
         next login/reload.",
    ));

    let default_mods = Entry::new();
    default_mods.set_text(&mods_to_text(&model.borrow().file.default_mods));
    default_mods.set_hexpand(true);
    default_mods.set_width_chars(20);
    {
        let model = model.clone();
        default_mods.connect_changed(move |e| {
            model.borrow_mut().file.default_mods = text_to_mods(&e.text());
        });
    }
    content.append(&w::row("Default mods", &default_mods));

    content.append(&section(Some("Bindings"), model, bindings_accessor(), rerender));

    let _ = status;
}

fn populate_multi_layout(content: &GBox, model: &Rc<RefCell<Model>>, rerender: &Rc<dyn Fn()>) {
    content.append(&w::hint(
        "Mods/Key/Action are the fields every bind needs. The last column \
         holds action-specific extras as inline JSON — e.g. \
         {\"command\": \"kitty\"}, {\"direction\": \"left\"}, \
         {\"workspace\": \"e+1\"}, {\"options\": {\"repeating\": true}} — \
         leave it blank for actions with none (close, exit, fullscreen, ...). \
         Applies on next login/reload.",
    ));

    let layout_names: Vec<String> = model.borrow().file.layouts.keys().cloned().collect();

    let top_row = GBox::new(Orientation::Horizontal, 12);
    top_row.append(&{
        let dd = DropDown::new(
            Some(StringList::new(&layout_names.iter().map(String::as_str).collect::<Vec<_>>())),
            Expression::NONE,
        );
        let cur = model.borrow().file.active_layout.clone();
        dd.set_selected(layout_names.iter().position(|n| *n == cur).unwrap_or(0) as u32);
        let model = model.clone();
        let layout_names = layout_names.clone();
        dd.connect_selected_notify(move |dd| {
            if let Some(name) = layout_names.get(dd.selected() as usize) {
                model.borrow_mut().file.active_layout = name.clone();
            }
        });
        w::row("Active layout", &dd)
    });
    content.append(&top_row);

    let default_mods = Entry::new();
    default_mods.set_text(&mods_to_text(&model.borrow().file.default_mods));
    default_mods.set_hexpand(true);
    default_mods.set_width_chars(20);
    {
        let model = model.clone();
        default_mods.connect_changed(move |e| {
            model.borrow_mut().file.default_mods = text_to_mods(&e.text());
        });
    }
    content.append(&w::row("Default mods", &default_mods));

    content.append(&section(Some("Media & function keys (globals)"), model, globals_accessor(), rerender));
    content.append(&section(Some("Common (every layout)"), model, common_accessor(), rerender));

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
                        m.file.layouts.remove(&name);
                        if m.file.active_layout == name {
                            m.file.active_layout = m.file.layouts.keys().next().cloned().unwrap_or_default();
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
        content.append(&section(None, model, layout_accessor(name.clone()), rerender));
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
            model.borrow_mut().file.layouts.entry(name).or_default();
            entry.set_text("");
            rerender();
        });
    }
    add_layout_row.append(&new_layout_name);
    add_layout_row.append(&add_layout_btn);
    content.append(&add_layout_row);
}

fn populate(content: &GBox, model: &Rc<RefCell<Model>>, status: &Label) {
    let rerender: Rc<dyn Fn()> = {
        let content = content.clone();
        let model = model.clone();
        let status = status.clone();
        Rc::new(move || rerender(&content, &model, &status))
    };

    let kind = model.borrow().kind;

    match kind {
        SchemaKind::Unknown => populate_unknown(content, status),
        SchemaKind::Flat => populate_flat(content, model, status, &rerender),
        SchemaKind::MultiLayout => populate_multi_layout(content, model, &rerender),
    }
}

pub fn build() -> GBox {
    let (outer, content) = w::view_scaffold("Keybinds");
    let (file, kind) = load();
    let model = Rc::new(RefCell::new(Model { kind, file }));

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
        save_btn.connect_clicked(move |_| {
            let m = model.borrow();
            match save(&m.file, m.kind) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative slice of BOS's real shipped `binds.json`
    /// (`iso/airootfs/etc/skel/.config/hypr/binds.json`, flat schema) —
    /// chosen to exercise the extra-field variety breadhelp reads (`label`,
    /// `category`, `demo_cmd`), an explicit `mods: []` override, a nested
    /// `options` object, and both integer and string `workspace` values.
    /// This is the fixture that would have caught the original bug: the
    /// editor mis-detecting this shape as MultiLayout and silently dropping
    /// the whole `bindings` array on save.
    const REAL_BOS_FLAT_FIXTURE: &str = r#"{
  "default_mods": ["SUPER"],
  "bindings": [
    { "action": "exec", "command": "kitty", "key": "RETURN", "label": "Open a terminal", "category": "apps" },
    { "action": "close", "key": "BACKSPACE", "label": "Close the focused window", "category": "windows" },
    { "action": "exec", "command": "breadbox", "key": "SPACE", "label": "Open the app launcher (breadbox)", "category": "apps", "demo_cmd": "breadbox" },
    { "action": "exec", "command": "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+", "key": "XF86AudioRaiseVolume", "mods": [], "options": { "locked": true, "repeating": true }, "label": "Volume up", "category": "media" },
    { "action": "focus", "workspace": 1, "key": "1", "label": "Switch to workspace 1", "category": "workspaces" },
    { "action": "focus", "workspace": "e+1", "key": "bracketright", "label": "Next workspace", "category": "workspaces" },
    { "action": "resize_dir", "x": 30, "y": 0, "key": "right", "mods": ["SUPER", "SHIFT"], "options": { "repeating": true }, "label": "Resize the focused window (grow right)", "category": "focus" },
    { "action": "drag", "key": "mouse:272", "options": { "mouse": true }, "label": "Move a window (drag)", "category": "mouse" }
  ]
}"#;

    fn parse(text: &str) -> (BindsFile, SchemaKind) {
        let kind = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(top)) => SchemaKind::detect(&top),
            _ => SchemaKind::Unknown,
        };
        let file: BindsFile = serde_json::from_str(text).unwrap_or_default();
        (file, kind)
    }

    #[test]
    fn detects_flat_schema_from_real_bos_binds_json() {
        let (_, kind) = parse(REAL_BOS_FLAT_FIXTURE);
        assert_eq!(kind, SchemaKind::Flat);
    }

    #[test]
    fn round_trips_real_bos_flat_binds_json_through_load_and_save() {
        let (file, kind) = parse(REAL_BOS_FLAT_FIXTURE);
        assert_eq!(kind, SchemaKind::Flat);

        let original: Value = serde_json::from_str(REAL_BOS_FLAT_FIXTURE).unwrap();
        let saved = to_json(&file, kind);

        // Flat save must emit EXACTLY {default_mods, bindings} — no
        // active_layout/globals/common/layouts keys leaking in.
        let saved_obj = saved.as_object().expect("flat save must be a JSON object");
        assert_eq!(
            saved_obj.keys().cloned().collect::<std::collections::BTreeSet<_>>(),
            ["default_mods", "bindings"].into_iter().map(String::from).collect(),
            "Flat schema must round-trip as exactly {{default_mods, bindings}}"
        );

        // The `bindings` array — and every per-bind extra field (label,
        // category, demo_cmd, mods, options, integer vs string workspace,
        // ...) — must survive the round trip semantically untouched.
        assert_eq!(saved["bindings"], original["bindings"]);
        assert_eq!(saved["default_mods"], original["default_mods"]);
    }

    #[test]
    fn round_trip_via_files_preserves_bindings_key_and_extras() {
        let dir = std::env::temp_dir().join(format!("bos-settings-keybinds-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("binds.json");
        std::fs::write(&path, REAL_BOS_FLAT_FIXTURE).unwrap();

        let (file, kind) = load_from(&path);
        assert_eq!(kind, SchemaKind::Flat);
        save_to(&path, &file, kind).unwrap();

        let saved_text = std::fs::read_to_string(&path).unwrap();
        let saved: Value = serde_json::from_str(&saved_text).unwrap();
        let original: Value = serde_json::from_str(REAL_BOS_FLAT_FIXTURE).unwrap();

        assert!(saved.get("bindings").is_some(), "bindings key must survive a load -> save round trip");
        assert_eq!(saved["bindings"], original["bindings"]);
        assert_eq!(saved["default_mods"], original["default_mods"]);

        // Backup safety net: a second save must leave `.bak` holding the
        // prior contents.
        save_to(&path, &file, kind).unwrap();
        let backup_path = dir.join("binds.json.bak");
        assert!(backup_path.exists(), "save must back up the previous file");
        let backup: Value = serde_json::from_str(&std::fs::read_to_string(&backup_path).unwrap()).unwrap();
        assert_eq!(backup["bindings"], original["bindings"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_and_round_trips_multi_layout_schema() {
        let text = r#"{
            "active_layout": "qwerty",
            "default_mods": ["SUPER"],
            "globals": [{ "action": "exec", "command": "kitty", "key": "RETURN" }],
            "common": [],
            "layouts": { "qwerty": [{ "action": "close", "key": "BACKSPACE" }] }
        }"#;
        let (file, kind) = parse(text);
        assert_eq!(kind, SchemaKind::MultiLayout);

        let saved = to_json(&file, kind);
        assert!(saved.get("bindings").is_none(), "MultiLayout save must not emit a flat `bindings` key");
        assert_eq!(saved["active_layout"], "qwerty");
        assert_eq!(saved["layouts"]["qwerty"][0]["action"], "close");
        assert_eq!(saved["globals"][0]["command"], "kitty");
    }

    #[test]
    fn unknown_schema_is_detected_and_refuses_to_save() {
        let text = r#"{ "some_other_shape": true }"#;
        let (file, kind) = parse(text);
        assert_eq!(kind, SchemaKind::Unknown);

        let dir = std::env::temp_dir().join(format!("bos-settings-keybinds-unknown-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("binds.json");

        let result = save_to(&path, &file, kind);
        assert!(result.is_err(), "save() must refuse when schema kind is Unknown");
        assert!(!path.exists(), "refusing to save must not create/touch the target file");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_defaults_to_flat_not_multi_layout() {
        let dir = std::env::temp_dir().join(format!("bos-settings-keybinds-missing-test-{}", std::process::id()));
        // Don't create the file at all.
        let path = dir.join("binds.json");
        let (_, kind) = load_from(&path);
        assert_eq!(kind, SchemaKind::Flat);
    }
}
