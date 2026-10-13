//! Display: live-connected-monitor readout (from `hyprctl monitors -j`) plus
//! an editor for `hypr/monitors.json`: the monitor *layout* Hyprland itself
//! reads at login. Like appearance.rs/autostart.rs, JSON has no comments to
//! preserve, but the file is still edited as a *merge*, never replaced:
//!
//! * rules are matched by output name, so a saved layout for an unplugged
//!   external display survives every change made while it is away;
//! * a rule's unknown keys (`mirror`, anything a later loader learns) are
//!   kept, only the keys this panel owns are overwritten;
//! * `transform` (rotation) and `vrr` are persisted next to mode, position
//!   and scale instead of being applied live and forgotten.
//!
//! `Default` here (the single wildcard rule) must stay in sync with
//! `scripts/display/monitors.lua`'s own `DEFAULT_MONITORS` fallback.
//!
//! NOTE: BOS's current `monitors.lua` loader only copies `output`, `mode`,
//! `position`, `scale` and `mirror` out of each entry, so `transform` and
//! `vrr` are written (and applied live) but not yet re-applied at login. The
//! loader needs `transform = entry.transform` / `vrr = entry.vrr` (type
//! checked) to close that loop; see the bos-settings PR for the exact patch.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::config;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MonitorRule {
    output: String,
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_position")]
    position: String,
    #[serde(default = "default_scale", deserialize_with = "de_scale")]
    scale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    transform: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vrr: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mirror: Option<String>,
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

/// The Lua loader accepts a numeric or a string `scale`; so must we, or one
/// hand-edited `"scale": 1.25` would make the whole file unreadable here.
fn de_scale<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    match Value::deserialize(d)? {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        _ => Ok(default_scale()),
    }
}

impl Default for MonitorRule {
    fn default() -> Self {
        Self {
            output: String::new(),
            mode: default_mode(),
            position: default_position(),
            scale: default_scale(),
            transform: None,
            vrr: None,
            mirror: None,
        }
    }
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/monitors.json")
}

fn hypr_path(name: &str) -> std::path::PathBuf {
    config::config_dir().join("hypr").join(name)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LiveMonitor {
    pub name: String,
    pub mode: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub scale: f64,
    pub transform: u32,
    pub available_modes: Vec<String>,
    /// Adaptive sync: 0 off, 1 on, 2 fullscreen only (Hyprland's `vrr`).
    #[serde(default)]
    pub vrr: u32,
    /// Name of the output this one mirrors, empty when it is not mirroring.
    #[serde(default)]
    pub mirror_of: String,
    /// Turned off in the compositor. Only `hyprctl monitors all` reports these.
    #[serde(default)]
    pub disabled: bool,
}

fn json_i32(v: &serde_json::Value, key: &str) -> Option<i32> {
    v.get(key)?
        .as_i64()
        .map(|n| n as i32)
        .or_else(|| v.get(key)?.as_f64().map(|n| n.round() as i32))
}

fn json_u32(v: &serde_json::Value, key: &str) -> Option<u32> {
    v.get(key)?
        .as_u64()
        .map(|n| n as u32)
        .or_else(|| v.get(key)?.as_f64().map(|n| n.round() as u32))
}

fn json_f64(v: &serde_json::Value, key: &str) -> Option<f64> {
    v.get(key)?
        .as_f64()
        .or_else(|| v.get(key)?.as_u64().map(|n| n as f64))
}

/// `hyprctl monitors -j` reports `vrr` as a bool on current Hyprland and as
/// an int on some builds; either way anything truthy means "on".
fn json_vrr(v: &serde_json::Value) -> u32 {
    match v.get("vrr") {
        Some(Value::Bool(b)) => u32::from(*b),
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0).min(2) as u32,
        _ => 0,
    }
}

/// Drop repeated entries but keep first-seen order. Hyprland lists some
/// modes twice (the same size and refresh from two EDID timings), and the
/// picker must never see the same mode twice.
fn dedupe_keep_order(items: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

/// Name of the output being mirrored, from a `mirrorOf` value. Current
/// Hyprland reports the source's numeric id ("0"), older builds a name;
/// "none", "" and "-1" mean the output is not mirroring.
fn resolve_mirror(
    value: Option<&Value>,
    ids: &std::collections::HashMap<String, String>,
) -> String {
    let raw = match value {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return String::new(),
    };
    if raw.is_empty() || raw == "none" || raw == "-1" {
        return String::new();
    }
    ids.get(&raw).cloned().unwrap_or(raw)
}

/// Turn the JSON of `hyprctl monitors [all] -j` into the panel's monitors.
/// Entries without a name or size are skipped, and a repeated output name is
/// kept only once.
pub(crate) fn parse_monitors(value: &serde_json::Value) -> Vec<LiveMonitor> {
    let Some(monitors) = value.as_array() else {
        return Vec::new();
    };
    // `mirrorOf` is the numeric id of the source output, not its name.
    let ids: std::collections::HashMap<String, String> = monitors
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_i64()?;
            Some((id.to_string(), m.get("name")?.as_str()?.to_string()))
        })
        .collect();
    let mut names = std::collections::HashSet::new();
    monitors
        .iter()
        .filter_map(|m| {
            let name = m.get("name")?.as_str()?.to_string();
            let width = json_u32(m, "width")?;
            let height = json_u32(m, "height")?;
            if !names.insert(name.clone()) {
                return None;
            }
            let refresh = json_f64(m, "refreshRate").unwrap_or(60.0);
            let x = json_i32(m, "x").unwrap_or(0);
            let y = json_i32(m, "y").unwrap_or(0);
            let scale = json_f64(m, "scale").unwrap_or(1.0).max(0.1);
            let transform = json_u32(m, "transform").unwrap_or(0);
            let available_modes = dedupe_keep_order(
                m.get("availableModes")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            );
            let mirror_of = resolve_mirror(m.get("mirrorOf"), &ids);
            let disabled = m.get("disabled").and_then(Value::as_bool).unwrap_or(false);
            Some(LiveMonitor {
                name,
                mode: format!("{width}x{height} @ {refresh:.0}Hz"),
                x,
                y,
                width,
                height,
                refresh,
                scale,
                transform,
                available_modes,
                vrr: json_vrr(m),
                mirror_of,
                disabled,
            })
        })
        .collect()
}

/// Every output Hyprland knows, including turned-off ones and mirrors, read
/// live from `hyprctl monitors all -j`.
pub(crate) fn read_all_monitors() -> Result<Vec<LiveMonitor>, String> {
    let value = bread_utils::proc::run_json(
        "hyprctl",
        &["monitors", "all", "-j"],
        std::time::Duration::from_secs(3),
    )
    .ok_or_else(|| "Hyprland did not answer `hyprctl monitors all -j`.".to_string())?;
    Ok(parse_monitors(&value))
}

/// The enabled outputs, with mirror relationships read from live state.
#[tauri::command]
pub fn get_live_monitors() -> Vec<LiveMonitor> {
    read_all_monitors()
        .unwrap_or_default()
        .into_iter()
        .filter(|m| !m.disabled)
        .collect()
}

/// The rule objects exactly as they sit in `monitors.json`, or empty when
/// the file is missing or unreadable (a corrupt file is backed up by
/// `atomic_write` before it is replaced, so nothing is lost silently).
fn read_rule_objects() -> Vec<Map<String, Value>> {
    let Ok(text) = std::fs::read_to_string(config_path()) else {
        return Vec::new();
    };
    parse_rule_objects(&text)
}

fn parse_rule_objects(text: &str) -> Vec<Map<String, Value>> {
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    root.get("monitors")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_object().cloned())
                .filter(|o| o.get("output").is_some_and(Value::is_string))
                .collect()
        })
        .unwrap_or_default()
}

fn write_rule_objects(rules: &[Map<String, Value>]) -> Result<(), String> {
    let mut root = Map::new();
    root.insert(
        "monitors".to_string(),
        Value::Array(rules.iter().cloned().map(Value::Object).collect()),
    );
    let json = serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}

pub(crate) fn lua_ident(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(format!("bad monitor name '{name}'"));
    }
    Ok(())
}

/// `WxH@R` with the refresh kept to two decimals (59.94 stays 59.94, 60.003
/// becomes 60) so a 59.94 Hz mode is not silently turned into 60 Hz.
pub(crate) fn mode_string(m: &LiveMonitor) -> String {
    let refresh = if m.refresh > 1.0 { m.refresh } else { 60.0 };
    let text = format!("{refresh:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{}x{}@{text}", m.width.max(1), m.height.max(1))
}

fn scale_string(m: &LiveMonitor) -> String {
    if (m.scale - 1.0).abs() < 0.001 {
        "1".to_string()
    } else {
        format!("{:.2}", m.scale)
    }
}

/// One `hl.monitor{}` call for a connected monitor. `vrr` is whatever the
/// panel was handed (the live value unless the user changed it); it is never
/// forced off.
pub(crate) fn lua_statement(m: &LiveMonitor) -> Result<String, String> {
    lua_ident(&m.name)?;
    if m.disabled {
        return Ok(format!(
            "hl.monitor({{ output = \"{}\", disabled = true }})",
            m.name
        ));
    }
    let mirror = if m.mirror_of.is_empty() {
        String::new()
    } else {
        lua_ident(&m.mirror_of)?;
        format!(", mirror = \"{}\"", m.mirror_of)
    };
    Ok(format!(
        "hl.monitor({{ output = \"{}\", mode = \"{}\", position = \"{}x{}\", scale = \"{}\", transform = {}, vrr = {}{mirror} }})",
        m.name,
        mode_string(m),
        m.x,
        m.y,
        scale_string(m),
        m.transform.min(7),
        m.vrr.min(2),
    ))
}

/// Merge the panel-owned keys of every live monitor into `existing`, by
/// output name. Rules for outputs that are not connected are left exactly as
/// they were; unknown keys on a matched rule (`mirror`, ...) survive. A
/// wildcard (`output = ""`) rule is moved first so the specific rules always
/// apply after it.
fn merge_rules(
    mut existing: Vec<Map<String, Value>>,
    live: &[LiveMonitor],
) -> Vec<Map<String, Value>> {
    for m in live {
        let pos = existing
            .iter()
            .position(|r| r.get("output").and_then(Value::as_str) == Some(m.name.as_str()));
        let rule = match pos {
            Some(i) => &mut existing[i],
            None => {
                let mut fresh = Map::new();
                fresh.insert("output".into(), Value::String(m.name.clone()));
                existing.push(fresh);
                existing.last_mut().expect("just pushed")
            }
        };
        rule.insert("mode".into(), Value::String(mode_string(m)));
        rule.insert("position".into(), Value::String(format!("{}x{}", m.x, m.y)));
        rule.insert("scale".into(), Value::String(scale_string(m)));
        rule.insert("transform".into(), Value::from(m.transform.min(7)));
        // A saved fullscreen-only value (2) must not be flattened to 1 just
        // because the panel only learns "on" from `hyprctl monitors`.
        let saved = rule.get("vrr").and_then(Value::as_u64).unwrap_or(0) as u32;
        let vrr = if m.vrr > 0 && saved == 2 {
            2
        } else {
            m.vrr.min(2)
        };
        rule.insert("vrr".into(), Value::from(vrr));
        if m.mirror_of.is_empty() {
            rule.remove("mirror");
        } else {
            rule.insert("mirror".into(), Value::String(m.mirror_of.clone()));
        }
    }
    let (mut wild, rest): (Vec<_>, Vec<_>) = existing
        .into_iter()
        .partition(|r| r.get("output").and_then(Value::as_str) == Some(""));
    wild.extend(rest);
    wild
}

/// Ask Hyprland (configured with hyprland.lua, so `hyprctl eval` works) to
/// run `lua`; maps its answers to readable errors. The reply must be "ok":
/// `hyprctl keyword` is refused under the Lua config yet still exits 0.
pub(crate) fn hyprctl_eval(lua: &str) -> Result<(), String> {
    let out = bread_utils::proc::run("hyprctl", &["eval", lua], std::time::Duration::from_secs(5));
    let stdout = out.stdout.trim().to_string();
    if stdout != "ok" {
        let low = stdout.to_lowercase();
        if low.contains("unknown request")
            || low.contains("unknown command")
            || low.contains("only supported with the lua config manager")
        {
            return Err("Live layout needs BOS Hyprland (hyprctl eval / hl.monitor).".into());
        }
        if stdout.is_empty() {
            let why = out.stderr.trim();
            return Err(if why.is_empty() {
                "hyprctl gave no answer.".into()
            } else {
                format!("hyprctl: {why}")
            });
        }
        return Err(format!("hyprctl: {stdout}"));
    }
    Ok(())
}

pub(crate) fn persist_layout(monitors: &[LiveMonitor]) -> Result<(), String> {
    for m in monitors {
        lua_ident(&m.name)?;
    }
    let merged = merge_rules(read_rule_objects(), monitors);
    let stmts = monitors
        .iter()
        .map(lua_statement)
        .collect::<Result<Vec<_>, _>>()?;
    super::desktop_preferences::install_lua_include(
        "bos-settings-display-overrides.lua",
        &format!("-- Managed by BOS Settings.\n{}\n", stmts.join("\n")),
    )?;
    write_rule_objects(&merged)
}

#[tauri::command]
pub fn get_monitor_rules() -> Vec<MonitorRule> {
    let rules: Vec<MonitorRule> = read_rule_objects()
        .into_iter()
        .filter_map(|o| serde_json::from_value(Value::Object(o)).ok())
        .collect();
    if rules.is_empty() {
        vec![MonitorRule::default()]
    } else {
        rules
    }
}

#[tauri::command]
pub fn save_monitor_rules(rules: Vec<MonitorRule>) -> Result<(), String> {
    let objects: Vec<Map<String, Value>> = rules
        .iter()
        .filter_map(|r| match serde_json::to_value(r) {
            Ok(Value::Object(o)) => Some(o),
            _ => None,
        })
        .collect();
    write_rule_objects(&objects)?;
    super::util::hypr_reload();
    Ok(())
}

/// Opens `hyprland.lua` in `$EDITOR` (nano if unset) inside a terminal;
/// spawning a TUI editor with no terminal to attach to is a silent no-op.
#[tauri::command]
pub fn open_hyprland_conf() {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let path = hypr_path("hyprland.lua");
    let _ = std::process::Command::new("kitty")
        .args(["-e", &editor])
        .arg(path)
        .spawn();
}

#[tauri::command]
pub fn open_keybinds_viewer() {
    let _ = std::process::Command::new("breadhelp").spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(name: &str) -> LiveMonitor {
        LiveMonitor {
            name: name.into(),
            mode: String::new(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1200,
            refresh: 60.003,
            scale: 1.0,
            transform: 0,
            available_modes: vec![],
            vrr: 0,
            mirror_of: String::new(),
            disabled: false,
        }
    }

    fn rules(json: &str) -> Vec<Map<String, Value>> {
        parse_rule_objects(json)
    }

    fn output(r: &Map<String, Value>) -> &str {
        r["output"].as_str().unwrap()
    }

    #[test]
    fn rotation_is_persisted_as_transform() {
        let mut m = mon("eDP-1");
        m.transform = 1;
        let merged = merge_rules(vec![], &[m]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0]["transform"], 1);
        assert_eq!(merged[0]["mode"], "1920x1200@60");
        assert_eq!(merged[0]["scale"], "1");
    }

    #[test]
    fn rules_for_disconnected_outputs_are_never_dropped() {
        let existing = rules(
            r#"{"monitors":[
                {"output":"eDP-1","mode":"1920x1200@60","position":"0x0","scale":"1"},
                {"output":"DP-3","mode":"2560x1440@144","position":"1920x0","scale":"1"}]}"#,
        );
        let mut m = mon("eDP-1");
        m.scale = 1.25;
        let merged = merge_rules(existing, &[m]);
        assert_eq!(merged.len(), 2);
        let dock = merged.iter().find(|r| output(r) == "DP-3").unwrap();
        assert_eq!(dock["mode"], "2560x1440@144");
        assert_eq!(dock["position"], "1920x0");
        let edp = merged.iter().find(|r| output(r) == "eDP-1").unwrap();
        assert_eq!(edp["scale"], "1.25");
    }

    #[test]
    fn unknown_keys_such_as_mirror_survive_a_merge() {
        let existing = rules(
            r#"{"monitors":[{"output":"HDMI-A-1","mode":"preferred","position":"auto","scale":"auto","mirror":"eDP-1","future":true}]}"#,
        );
        let mut m = mon("HDMI-A-1");
        m.mirror_of = "eDP-1".into();
        let merged = merge_rules(existing, &[m]);
        assert_eq!(merged[0]["mirror"], "eDP-1");
        assert_eq!(merged[0]["future"], true);
    }

    #[test]
    fn mirror_is_removed_when_the_output_stops_mirroring() {
        let existing = rules(r#"{"monitors":[{"output":"HDMI-A-1","mirror":"eDP-1"}]}"#);
        let merged = merge_rules(existing, &[mon("HDMI-A-1")]);
        assert!(!merged[0].contains_key("mirror"));
    }

    #[test]
    fn vrr_is_persisted_and_fullscreen_only_is_not_flattened() {
        let mut on = mon("eDP-1");
        on.vrr = 1;
        assert_eq!(merge_rules(vec![], &[on.clone()])[0]["vrr"], 1);
        let saved = rules(r#"{"monitors":[{"output":"eDP-1","vrr":2}]}"#);
        assert_eq!(merge_rules(saved, &[on])[0]["vrr"], 2);
        let off = mon("eDP-1");
        let saved = rules(r#"{"monitors":[{"output":"eDP-1","vrr":2}]}"#);
        assert_eq!(merge_rules(saved, &[off])[0]["vrr"], 0);
    }

    #[test]
    fn wildcard_rule_is_kept_and_sorted_first() {
        let existing = rules(
            r#"{"monitors":[{"output":"DP-3"},{"output":"","mode":"preferred","position":"auto","scale":"auto"}]}"#,
        );
        let merged = merge_rules(existing, &[mon("eDP-1")]);
        assert_eq!(merged.len(), 3);
        assert_eq!(output(&merged[0]), "");
    }

    #[test]
    fn numeric_scale_in_the_file_is_tolerated() {
        let parsed: MonitorRule =
            serde_json::from_str(r#"{"output":"eDP-1","scale":1.25}"#).unwrap();
        assert_eq!(parsed.scale, "1.25");
    }

    #[test]
    fn corrupt_or_wrong_shaped_file_reads_as_no_rules() {
        assert!(parse_rule_objects("not json").is_empty());
        assert!(parse_rule_objects(r#"{"monitors": 3}"#).is_empty());
        assert!(parse_rule_objects(r#"{"monitors": [1, {"mode": "x"}]}"#).is_empty());
    }

    #[test]
    fn lua_statement_carries_transform_and_live_vrr() {
        let mut m = mon("eDP-1");
        m.transform = 3;
        m.vrr = 1;
        m.x = 1920;
        let s = lua_statement(&m).unwrap();
        assert!(s.contains("transform = 3"));
        assert!(s.contains("vrr = 1"));
        assert!(s.contains("position = \"1920x0\""));
        assert!(!s.contains("vrr = false"));
    }

    #[test]
    fn lua_statement_rejects_bad_names() {
        assert!(lua_statement(&mon("eDP-1\"); os.execute(\"x")).is_err());
        let mut m = mon("eDP-1");
        m.mirror_of = "a\"b".into();
        assert!(lua_statement(&m).is_err());
    }

    const FIXTURE: &str = include_str!("testdata/hyprctl-monitors-all.json");

    fn fixture_monitors() -> Vec<LiveMonitor> {
        parse_monitors(&serde_json::from_str(FIXTURE).unwrap())
    }

    #[test]
    fn duplicate_modes_are_removed_keeping_order() {
        let monitors = fixture_monitors();
        for name in ["DVI-I-1", "DVI-I-2"] {
            let m = monitors.iter().find(|m| m.name == name).unwrap();
            assert_eq!(m.available_modes.len(), 25, "{name}");
            let unique: std::collections::HashSet<_> = m.available_modes.iter().collect();
            assert_eq!(unique.len(), m.available_modes.len(), "{name}");
            assert_eq!(m.available_modes[0], "1920x1080@60.00Hz");
            assert_eq!(m.available_modes[1], "1920x1080@59.94Hz");
            assert_eq!(m.available_modes.last().unwrap(), "720x400@70.08Hz");
        }
        let edp = monitors.iter().find(|m| m.name == "eDP-1").unwrap();
        assert_eq!(edp.available_modes.len(), 11);
    }

    #[test]
    fn dedupe_keeps_first_occurrence_order() {
        let out = dedupe_keep_order(["b", "a", "b", "c", "a"].map(String::from));
        assert_eq!(out, ["b", "a", "c"]);
    }

    #[test]
    fn repeated_output_names_are_kept_once() {
        let v = serde_json::json!([
            {"name": "A", "width": 1, "height": 1},
            {"name": "A", "width": 2, "height": 2}
        ]);
        let parsed = parse_monitors(&v);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].width, 1);
    }
}
