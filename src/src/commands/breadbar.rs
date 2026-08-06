//! breadbar/style.css — CSS overrides for the bar. No systemd unit (breadbar
//! is launched directly by hyprland.lua's exec-once); SIGHUP is its own
//! documented live-reload mechanism.
//!
//! The file has a fixed, hand-written structure (see the shipped template's
//! comments), so the common properties users actually tweak — font, bar
//! chrome, workspace indicator, spacing, tray/notification radii — are
//! exposed as a typed `BreadbarStyle` struct. `get`/`set_value` locate a
//! known `selector { ... }` block and rewrite just one declaration's value
//! inside it, leaving comments, ordering, and every unmodeled property
//! (font-weight, per-element opacities, notification padding, etc.)
//! untouched. Anything not modeled here stays reachable via the raw
//! `get`/`save_breadbar_css` pair, kept as an "Advanced" escape hatch.

use regex::{Captures, Regex};

use super::config;

fn css_path() -> std::path::PathBuf {
    config::config_dir().join("breadbar/style.css")
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BreadbarStyle {
    pub font_family: String,
    pub font_size: u32,
    pub bar_border_radius: u32,
    pub bar_padding: u32,
    pub workspace_inactive_opacity: f64,
    pub workspace_font_size: u32,
    pub stat_gap: u32,
    pub tray_icon_size: u32,
    pub notification_border_radius: u32,
}

fn find_block(css: &str, selector: &str) -> Option<(usize, usize)> {
    let pat = format!(r"(?m)^\s*{}\s*\{{", regex::escape(selector));
    let re = Regex::new(&pat).ok()?;
    let m = re.find(css)?;
    let body_start = m.end();
    let body_end = body_start + css[body_start..].find('}')?;
    Some((body_start, body_end))
}

fn get_value(css: &str, selector: &str, property: &str) -> Option<String> {
    let (start, end) = find_block(css, selector)?;
    let body = &css[start..end];
    let re = Regex::new(&format!(r"(?m)^\s*{}\s*:\s*([^;]+);", regex::escape(property))).ok()?;
    Some(re.captures(body)?.get(1)?.as_str().trim().to_string())
}

fn set_value(css: &str, selector: &str, property: &str, new_value: &str) -> Option<String> {
    let (start, end) = find_block(css, selector)?;
    let body = &css[start..end];
    let re = Regex::new(&format!(r"(?m)(^\s*{}\s*:\s*)([^;]+)(;)", regex::escape(property))).ok()?;
    if !re.is_match(body) {
        return None;
    }
    let new_body = re
        .replace(body, |caps: &Captures| format!("{}{}{}", &caps[1], new_value, &caps[3]))
        .into_owned();
    Some(format!("{}{}{}", &css[..start], new_body, &css[end..]))
}

fn strip_px(v: &str) -> u32 {
    v.trim().trim_end_matches("px").trim().parse().unwrap_or(0)
}

#[tauri::command]
pub fn get_breadbar_css() -> String {
    std::fs::read_to_string(css_path()).unwrap_or_default()
}

/// Saves the CSS and sends breadbar SIGHUP to live-reload it. Returns
/// whether the reload signal was actually delivered (`false` just means
/// breadbar isn't running — the file is saved either way).
#[tauri::command]
pub fn save_breadbar_css(css: String) -> Result<bool, String> {
    config::atomic_write(&css_path(), &css).map_err(|e| e.to_string())?;
    reload_breadbar()
}

#[tauri::command]
pub fn get_breadbar_style() -> BreadbarStyle {
    let css = std::fs::read_to_string(css_path()).unwrap_or_default();
    BreadbarStyle {
        font_family: get_value(&css, "*", "font-family")
            .and_then(|v| v.split(',').next().map(|s| s.trim().trim_matches(['\'', '"']).to_string()))
            .unwrap_or_else(|| "Varela Round".to_string()),
        font_size: get_value(&css, "*", "font-size").map(|v| strip_px(&v)).unwrap_or(14),
        bar_border_radius: get_value(&css, "window.breadbar", "border-radius")
            .map(|v| strip_px(&v))
            .unwrap_or(0),
        bar_padding: get_value(&css, "window.breadbar", "padding").map(|v| strip_px(&v)).unwrap_or(0),
        workspace_inactive_opacity: get_value(&css, ".workspace-btn", "opacity")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0.45),
        workspace_font_size: get_value(&css, ".workspace-btn", "font-size")
            .map(|v| strip_px(&v))
            .unwrap_or(20),
        stat_gap: get_value(&css, ".stat-pair", "margin-right").map(|v| strip_px(&v)).unwrap_or(12),
        tray_icon_size: get_value(&css, ".tray-btn image", "-gtk-icon-size")
            .map(|v| strip_px(&v))
            .unwrap_or(16),
        notification_border_radius: get_value(&css, "window.breadbar-notification", "border-radius")
            .map(|v| strip_px(&v))
            .unwrap_or(6),
    }
}

#[tauri::command]
pub fn save_breadbar_style(style: BreadbarStyle) -> Result<bool, String> {
    let mut css = std::fs::read_to_string(css_path()).unwrap_or_default();
    let px = |n: u32| format!("{n}px");

    let edits: [(&str, &str, String); 9] = [
        ("*", "font-family", format!("'{}', sans-serif", style.font_family)),
        ("*", "font-size", px(style.font_size)),
        ("window.breadbar", "border-radius", px(style.bar_border_radius)),
        ("window.breadbar", "padding", px(style.bar_padding)),
        (".workspace-btn", "opacity", style.workspace_inactive_opacity.to_string()),
        (".workspace-btn", "font-size", px(style.workspace_font_size)),
        (".stat-pair", "margin-right", px(style.stat_gap)),
        (".tray-btn image", "-gtk-icon-size", px(style.tray_icon_size)),
        ("window.breadbar-notification", "border-radius", px(style.notification_border_radius)),
    ];

    for (selector, property, value) in &edits {
        if let Some(updated) = set_value(&css, selector, property, value) {
            css = updated;
        }
    }

    config::atomic_write(&css_path(), &css).map_err(|e| e.to_string())?;
    reload_breadbar()
}

fn reload_breadbar() -> Result<bool, String> {
    Ok(std::process::Command::new("pkill")
        .args(["-HUP", "-x", "breadbar"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false))
}
