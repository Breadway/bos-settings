//! Display layout editing with a keep-or-revert guard.

use std::collections::HashSet;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Monitor {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub scale: f64,
    pub transform: u32,
    pub vrr: u32,
    pub mirror_of: String,
    pub disabled: bool,
    pub available_modes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Pending {
    pub id: u64,
    pub seconds_left: u64,
}

struct Guard {
    next_id: u64,
    pending: Option<(u64, u64, Vec<Monitor>)>,
}

fn guard() -> &'static Mutex<Guard> {
    static GUARD: OnceLock<Mutex<Guard>> = OnceLock::new();
    GUARD.get_or_init(|| {
        Mutex::new(Guard {
            next_id: 1,
            pending: None,
        })
    })
}

pub fn load() -> Vec<Monitor> {
    let Some(text) = text("hyprctl", &["monitors", "all", "-j"]) else {
        return Vec::new();
    };
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    rows.as_array()
        .map(|rows| rows.iter().filter_map(parse_monitor).collect())
        .unwrap_or_default()
}

fn parse_monitor(row: &serde_json::Value) -> Option<Monitor> {
    Some(Monitor {
        name: row.get("name")?.as_str()?.to_string(),
        x: row.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        y: row.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        width: row.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        height: row.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        refresh: row.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(60.0),
        scale: row.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0),
        transform: row.get("transform").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        vrr: row.get("vrr").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        mirror_of: row
            .get("mirrorOf")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        disabled: row.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false),
        available_modes: row
            .get("availableModes")
            .and_then(|v| v.as_array())
            .map(|modes| {
                modes
                    .iter()
                    .filter_map(|mode| mode.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn text(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn current_mode(monitor: &Monitor) -> String {
    format!(
        "{}x{}@{:.2}",
        monitor.width.max(1),
        monitor.height.max(1),
        if monitor.refresh > 1.0 {
            monitor.refresh
        } else {
            60.0
        }
    )
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

pub fn validate(layout: &[Monitor]) -> Result<(), String> {
    if layout.is_empty() {
        return Err("There are no displays to apply.".into());
    }
    let mut seen = HashSet::new();
    for monitor in layout {
        if !valid_name(&monitor.name) {
            return Err(format!("bad monitor name '{}'", monitor.name));
        }
        if !seen.insert(monitor.name.as_str()) {
            return Err(format!("{} is listed twice.", monitor.name));
        }
        if monitor.disabled {
            continue;
        }
        if monitor.width == 0 || monitor.height == 0 || !monitor.refresh.is_finite() {
            return Err(format!("{} has no usable resolution.", monitor.name));
        }
        if !monitor.scale.is_finite() || !(0.1..=10.0).contains(&monitor.scale) {
            return Err(format!("{} has an unusable scale.", monitor.name));
        }
        if monitor.transform > 7 || monitor.vrr > 2 {
            return Err(format!("{} has an unusable rotation or sync value.", monitor.name));
        }
    }
    if layout.iter().all(|monitor| monitor.disabled) {
        return Err("At least one display has to stay on.".into());
    }
    for monitor in layout.iter().filter(|m| !m.disabled && !m.mirror_of.is_empty()) {
        if !valid_name(&monitor.mirror_of) || monitor.mirror_of == monitor.name {
            return Err(format!("{} cannot mirror {}.", monitor.name, monitor.mirror_of));
        }
        match layout.iter().find(|other| other.name == monitor.mirror_of) {
            None => {
                return Err(format!(
                    "{} cannot mirror {}: that display is not connected.",
                    monitor.name, monitor.mirror_of
                ))
            }
            Some(source) if source.disabled => {
                return Err(format!(
                    "{} cannot mirror {}: that display is turned off.",
                    monitor.name, monitor.mirror_of
                ))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

pub fn statement(monitor: &Monitor) -> Result<String, String> {
    if !valid_name(&monitor.name) {
        return Err(format!("bad monitor name '{}'", monitor.name));
    }
    if monitor.disabled {
        return Ok(format!(
            "hl.monitor({{ output = \"{}\", disabled = true }})",
            monitor.name
        ));
    }
    let mirror = if monitor.mirror_of.is_empty() {
        String::new()
    } else {
        if !valid_name(&monitor.mirror_of) {
            return Err(format!("bad monitor name '{}'", monitor.mirror_of));
        }
        format!(", mirror = \"{}\"", monitor.mirror_of)
    };
    let scale = if (monitor.scale - 1.0).abs() < 0.001 {
        "1".to_string()
    } else {
        format!("{:.2}", monitor.scale)
    };
    Ok(format!(
        "hl.monitor({{ output = \"{}\", mode = \"{}\", position = \"{}x{}\", scale = \"{}\", transform = {}, vrr = {}{} }})",
        monitor.name,
        current_mode(monitor),
        monitor.x,
        monitor.y,
        scale,
        monitor.transform.min(7),
        monitor.vrr.min(2),
        mirror
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonitorRule {
    pub output: String,
    pub mode: String,
    pub position: String,
    pub scale: String,
    pub transform: Option<u32>,
    pub vrr: Option<u32>,
    pub mirror: Option<String>,
}

fn rules_path() -> std::path::PathBuf {
    crate::config::config_dir().join("hypr/monitors.json")
}

fn read_rule_objects() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let Ok(text) = std::fs::read_to_string(rules_path()) else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    root.get("monitors")
        .and_then(|value| value.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.as_object().cloned())
                .filter(|row| row.get("output").is_some_and(serde_json::Value::is_string))
                .collect()
        })
        .unwrap_or_default()
}

fn rule_from_object(object: &serde_json::Map<String, serde_json::Value>) -> Option<MonitorRule> {
    let output = object.get("output")?.as_str()?.to_string();
    let scale = match object.get("scale") {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Number(number)) => number.to_string(),
        _ => "auto".into(),
    };
    Some(MonitorRule {
        output,
        mode: object
            .get("mode")
            .and_then(|value| value.as_str())
            .unwrap_or("preferred")
            .into(),
        position: object
            .get("position")
            .and_then(|value| value.as_str())
            .unwrap_or("auto")
            .into(),
        scale,
        transform: object
            .get("transform")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32),
        vrr: object
            .get("vrr")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32),
        mirror: object
            .get("mirror")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

pub fn get_monitor_rules() -> Vec<MonitorRule> {
    let rules: Vec<MonitorRule> = read_rule_objects()
        .iter()
        .filter_map(rule_from_object)
        .collect();
    if rules.is_empty() {
        vec![MonitorRule {
            output: String::new(),
            mode: "preferred".into(),
            position: "auto".into(),
            scale: "auto".into(),
            transform: None,
            vrr: None,
            mirror: None,
        }]
    } else {
        rules
    }
}

pub fn save_monitor_rules(rules: &[MonitorRule]) -> Result<(), String> {
    let objects: Vec<serde_json::Value> = rules
        .iter()
        .map(|rule| {
            let mut object = serde_json::Map::new();
            object.insert("output".into(), rule.output.clone().into());
            object.insert("mode".into(), rule.mode.clone().into());
            object.insert("position".into(), rule.position.clone().into());
            object.insert("scale".into(), rule.scale.clone().into());
            if let Some(transform) = rule.transform {
                object.insert("transform".into(), transform.into());
            }
            if let Some(vrr) = rule.vrr {
                object.insert("vrr".into(), vrr.into());
            }
            if let Some(mirror) = &rule.mirror {
                object.insert("mirror".into(), mirror.clone().into());
            }
            serde_json::Value::Object(object)
        })
        .collect();
    let root = serde_json::json!({ "monitors": objects });
    crate::config::atomic_write(
        &rules_path(),
        &serde_json::to_string_pretty(&root).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    crate::util::hypr_reload();
    Ok(())
}

pub fn write_layout(layout: &[Monitor]) -> Result<(), String> {
    validate(layout)?;
    let mut ordered: Vec<&Monitor> = layout.iter().collect();
    ordered.sort_by_key(|monitor| !monitor.mirror_of.is_empty());
    for monitor in ordered {
        let code = statement(monitor)?;
        let out = Command::new("hyprctl")
            .args(["eval", &code])
            .output()
            .map_err(|error| error.to_string())?;
        let answer = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !out.status.success() || !answer.to_ascii_lowercase().contains("ok") {
            return Err(format!("{} was not applied: {}", monitor.name, answer.trim()));
        }
    }
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn apply(layout: Vec<Monitor>, seconds: u64) -> Result<u64, String> {
    let seconds = seconds.clamp(5, 60);
    let previous = load();
    let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
    if let Some((old_id, _, old_layout)) = state.pending.take() {
        let _ = old_id;
        let _ = write_layout(&old_layout);
    }
    let id = state.next_id;
    state.next_id += 1;
    let deadline = now_ms() + seconds.saturating_mul(1000);
    let record = crate::display_rescue::record_path();
    if let Some(path) = &record {
        let _ = crate::display_rescue::write_record(
            path,
            &crate::display_rescue::Record {
                id,
                pid: std::process::id(),
                deadline_ms: deadline,
                before: previous.clone(),
            },
        );
    }
    if let Err(error) = write_layout(&layout) {
        if let Some(path) = &record {
            crate::display_rescue::clear_record(path, id);
        }
        return Err(error);
    }
    state.pending = Some((id, deadline, previous));
    drop(state);
    let _ = crate::display_rescue::spawn_watchdog(id);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
            let Some((pending_id, deadline, previous)) = state.pending.clone() else {
                return;
            };
            if pending_id != id {
                return;
            }
            if now_ms() >= deadline {
                state.pending = None;
                drop(state);
                let _ = write_layout(&previous);
                if let Some(path) = crate::display_rescue::record_path() {
                    crate::display_rescue::clear_record(&path, id);
                }
                return;
            }
        }
    });
    Ok(id)
}

pub fn keep(id: u64) -> Result<(), String> {
    let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
    match &state.pending {
        Some((pending, _, _)) if *pending == id => {
            state.pending = None;
            if let Some(path) = crate::display_rescue::record_path() {
                crate::display_rescue::clear_record(&path, id);
            }
            Ok(())
        }
        _ => Err("That display change is no longer waiting.".into()),
    }
}

pub fn revert(id: u64) -> Result<(), String> {
    let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
    match state.pending.take() {
        Some((pending, _, previous)) if pending == id => {
            drop(state);
            if let Some(path) = crate::display_rescue::record_path() {
                crate::display_rescue::clear_record(&path, id);
            }
            write_layout(&previous)
        }
        _ => Err("That display change is no longer waiting.".into()),
    }
}

/// Called on exit: a layout nobody confirmed is put back before the
/// window goes away.
pub fn revert_on_exit() {
    let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
    if let Some((id, _, previous)) = state.pending.take() {
        drop(state);
        let _ = write_layout(&previous);
        if let Some(path) = crate::display_rescue::record_path() {
            crate::display_rescue::clear_record(&path, id);
        }
    }
}

pub fn status() -> Pending {
    let state = guard().lock().unwrap_or_else(|e| e.into_inner());
    state
        .pending
        .as_ref()
        .map(|(id, deadline, _)| Pending {
            id: *id,
            seconds_left: deadline.saturating_sub(now_ms()).div_ceil(1000),
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str) -> Monitor {
        Monitor {
            name: name.into(),
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            refresh: 60.0,
            scale: 1.0,
            transform: 0,
            vrr: 0,
            mirror_of: String::new(),
            disabled: false,
            available_modes: vec!["1920x1080@60.00".into()],
        }
    }

    #[test]
    fn statement_uses_the_lua_api() {
        let text = statement(&monitor("eDP-1")).unwrap();
        assert!(text.starts_with("hl.monitor("));
        assert!(text.contains("mode = \"1920x1080@60.00\""));
        assert!(text.contains("position = \"0x0\""));
    }

    #[test]
    fn invalid_layouts_are_rejected() {
        assert!(validate(&[]).is_err());
        let mut only = monitor("eDP-1");
        only.disabled = true;
        assert!(validate(&[only]).is_err());
        let mut bad = monitor("eDP-1");
        bad.mirror_of = "HDMI-A-1".into();
        assert!(validate(&[bad]).is_err());
    }

    #[test]
    fn scale_and_mirror_render() {
        let mut left = monitor("eDP-1");
        left.scale = 1.5;
        left.x = 0;
        let mut right = monitor("HDMI-A-1");
        right.mirror_of = "eDP-1".into();
        let text = statement(&left).unwrap();
        assert!(text.contains("scale = \"1.50\""));
        let mirror = statement(&right).unwrap();
        assert!(mirror.contains("mirror = \"eDP-1\""));
    }
}
