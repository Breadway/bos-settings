//! Display layout editing with a keep-or-revert guard.

use std::collections::HashSet;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
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

fn write_layout(layout: &[Monitor]) -> Result<(), String> {
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
    let previous = load();
    let mut state = guard().lock().unwrap_or_else(|e| e.into_inner());
    if let Some((old_id, _, old_layout)) = state.pending.take() {
        let _ = old_id;
        let _ = write_layout(&old_layout);
    }
    write_layout(&layout)?;
    let id = state.next_id;
    state.next_id += 1;
    let deadline = now_ms() + seconds.saturating_mul(1000);
    state.pending = Some((id, deadline, previous));
    drop(state);
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
            write_layout(&previous)
        }
        _ => Err("That display change is no longer waiting.".into()),
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
