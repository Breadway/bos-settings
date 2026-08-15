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
//! `SchemaKind` detects which shape is actually on disk (from the top-level
//! key set) and `save()` always emits that SAME shape back — see
//! `SchemaKind::detect` and `save_to`. Loading a real BOS (Flat) file under
//! the wrong assumption and saving it back would silently drop the
//! `bindings` key entirely — still valid JSON, so the Lua `pcall`
//! failsafes on the reading side would never catch it.
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

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::config;

/// Which on-disk shape `binds.json` was loaded as. Detected once at load
/// time from the top-level key set present in the JSON, then pinned for the
/// lifetime of the editor session (round-tripped to the frontend and back
/// on save) so `save()` always writes back the same shape it read,
/// regardless of what the in-memory model happens to have populated.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaKind {
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
pub struct Bind {
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
    /// `demo_cmd`, and any action shape not yet invented. Edited on the
    /// frontend as compact inline JSON. This flatten is what keeps
    /// breadhelp's `label`/`category`/`demo_cmd` fields — which this
    /// editor's UI has no dedicated widgets for — alive across a full
    /// load/save round trip instead of being silently dropped.
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BindsFile {
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

/// What the frontend fetches once at load: which shape was detected, plus
/// the data itself. Sent back verbatim to `save_keybinds` so a stray code
/// path on either side can't accidentally save `file` without knowing which
/// shape it's supposed to come back out as.
#[derive(Serialize)]
pub struct BindsPayload {
    kind: SchemaKind,
    file: BindsFile,
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/binds.json")
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
    config::atomic_write(path, &text)
}

fn save(f: &BindsFile, kind: SchemaKind) -> std::io::Result<()> {
    save_to(&config_path(), f, kind)
}

/// One breadshot `exec` bind as binds.json stored it. Used by the
/// Screenshots panel (read-only); editing still happens here.
pub(crate) struct ShotBindRaw {
    pub mods: Option<Vec<String>>,
    pub key: Option<String>,
    pub command: String,
    pub default_mods: Vec<String>,
}

pub(crate) fn breadshot_binds() -> Vec<ShotBindRaw> {
    let (file, _kind) = load();
    let default_mods = file.default_mods.clone();
    let mut out = Vec::new();
    let mut push = |binds: &[Bind]| {
        for b in binds {
            if b.action != "exec" {
                continue;
            }
            let Some(cmd) = b.extra.get("command").and_then(|v| v.as_str()) else {
                continue;
            };
            if !cmd
                .split_whitespace()
                .next()
                .is_some_and(|bin| bin == "breadshot" || bin.ends_with("/breadshot"))
            {
                continue;
            }
            out.push(ShotBindRaw {
                mods: b.mods.clone(),
                key: b.key.clone(),
                command: cmd.to_string(),
                default_mods: default_mods.clone(),
            });
        }
    };
    push(&file.bindings);
    push(&file.globals);
    push(&file.common);
    for binds in file.layouts.values() {
        push(binds);
    }
    out
}

#[tauri::command]
pub fn get_keybinds() -> BindsPayload {
    let (file, kind) = load();
    BindsPayload { kind, file }
}

#[tauri::command]
pub fn save_keybinds(file: BindsFile, kind: SchemaKind) -> Result<(), String> {
    save(&file, kind).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative slice of BOS's real shipped `binds.json`
    /// (`iso/airootfs/etc/skel/.config/hypr/binds.json`, flat schema) —
    /// chosen to exercise the extra-field variety breadhelp reads (`label`,
    /// `category`, `demo_cmd`), an explicit `mods: []` override, a nested
    /// `options` object, and both integer and string `workspace` values.
    /// This is the fixture that would have caught the original GTK-editor
    /// bug: mis-detecting this shape as MultiLayout and silently dropping
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
            saved_obj
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            ["default_mods", "bindings"]
                .into_iter()
                .map(String::from)
                .collect(),
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
        let dir =
            std::env::temp_dir().join(format!("bos-settings-keybinds-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("binds.json");
        std::fs::write(&path, REAL_BOS_FLAT_FIXTURE).unwrap();

        let (file, kind) = load_from(&path);
        assert_eq!(kind, SchemaKind::Flat);
        save_to(&path, &file, kind).unwrap();

        let saved_text = std::fs::read_to_string(&path).unwrap();
        let saved: Value = serde_json::from_str(&saved_text).unwrap();
        let original: Value = serde_json::from_str(REAL_BOS_FLAT_FIXTURE).unwrap();

        assert!(
            saved.get("bindings").is_some(),
            "bindings key must survive a load -> save round trip"
        );
        assert_eq!(saved["bindings"], original["bindings"]);
        assert_eq!(saved["default_mods"], original["default_mods"]);

        // Backup safety net: a second save must leave `.bak` holding the
        // prior contents.
        save_to(&path, &file, kind).unwrap();
        let backup_path = dir.join("binds.json.bak");
        assert!(backup_path.exists(), "save must back up the previous file");
        let backup: Value =
            serde_json::from_str(&std::fs::read_to_string(&backup_path).unwrap()).unwrap();
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
        assert!(
            saved.get("bindings").is_none(),
            "MultiLayout save must not emit a flat `bindings` key"
        );
        assert_eq!(saved["active_layout"], "qwerty");
        assert_eq!(saved["layouts"]["qwerty"][0]["action"], "close");
        assert_eq!(saved["globals"][0]["command"], "kitty");
    }

    #[test]
    fn unknown_schema_is_detected_and_refuses_to_save() {
        let text = r#"{ "some_other_shape": true }"#;
        let (file, kind) = parse(text);
        assert_eq!(kind, SchemaKind::Unknown);

        let dir = std::env::temp_dir().join(format!(
            "bos-settings-keybinds-unknown-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("binds.json");

        let result = save_to(&path, &file, kind);
        assert!(
            result.is_err(),
            "save() must refuse when schema kind is Unknown"
        );
        assert!(
            !path.exists(),
            "refusing to save must not create/touch the target file"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_defaults_to_flat_not_multi_layout() {
        let dir = std::env::temp_dir().join(format!(
            "bos-settings-keybinds-missing-test-{}",
            std::process::id()
        ));
        // Don't create the file at all.
        let path = dir.join("binds.json");
        let (_, kind) = load_from(&path);
        assert_eq!(kind, SchemaKind::Flat);
    }
}
