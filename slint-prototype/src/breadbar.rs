//! breadbar/style.css: the typed style values plus the raw escape hatch.
//! Edits rewrite one declaration inside a known block, so comments, order
//! and unmodelled properties survive.

use std::path::PathBuf;

use regex::{Captures, Regex};

use crate::config;

fn css_path() -> PathBuf {
    config::config_dir().join("breadbar/style.css")
}

#[derive(Clone)]
pub struct Style {
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
    let pattern = format!(r"(?m)^\s*{}\s*\{{", regex::escape(selector));
    let regex = Regex::new(&pattern).ok()?;
    let found = regex.find(css)?;
    let start = found.end();
    let end = start + css[start..].find('}')?;
    Some((start, end))
}

fn get_value(css: &str, selector: &str, property: &str) -> Option<String> {
    let (start, end) = find_block(css, selector)?;
    let body = &css[start..end];
    let regex = Regex::new(&format!(
        r"(?m)^\s*{}\s*:\s*([^;]+);",
        regex::escape(property)
    ))
    .ok()?;
    Some(regex.captures(body)?.get(1)?.as_str().trim().to_string())
}

fn set_value(css: &str, selector: &str, property: &str, new_value: &str) -> Option<String> {
    let (start, end) = find_block(css, selector)?;
    let body = &css[start..end];
    let regex = Regex::new(&format!(
        r"(?m)(^\s*{}\s*:\s*)([^;]+)(;)",
        regex::escape(property)
    ))
    .ok()?;
    if !regex.is_match(body) {
        return None;
    }
    let new_body = regex
        .replace(body, |captures: &Captures| {
            format!("{}{}{}", &captures[1], new_value, &captures[3])
        })
        .into_owned();
    Some(format!("{}{}{}", &css[..start], new_body, &css[end..]))
}

fn strip_px(value: &str) -> u32 {
    value.trim().trim_end_matches("px").trim().parse().unwrap_or(0)
}

pub fn get_css() -> String {
    std::fs::read_to_string(css_path()).unwrap_or_default()
}

pub fn save_css(css: &str) -> Result<bool, String> {
    config::atomic_write(&css_path(), css).map_err(|error| error.to_string())?;
    reload()
}

pub fn load() -> Style {
    let css = get_css();
    Style {
        font_family: get_value(&css, "*", "font-family")
            .and_then(|value| {
                value
                    .split(',')
                    .next()
                    .map(|part| part.trim().trim_matches(['\'', '"']).to_string())
            })
            .unwrap_or_else(|| "Varela Round".into()),
        font_size: get_value(&css, "*", "font-size")
            .map(|value| strip_px(&value))
            .unwrap_or(14),
        bar_border_radius: get_value(&css, "window.breadbar", "border-radius")
            .map(|value| strip_px(&value))
            .unwrap_or(0),
        bar_padding: get_value(&css, "window.breadbar", "padding")
            .map(|value| strip_px(&value))
            .unwrap_or(0),
        workspace_inactive_opacity: get_value(&css, ".workspace-btn", "opacity")
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0.45),
        workspace_font_size: get_value(&css, ".workspace-btn", "font-size")
            .map(|value| strip_px(&value))
            .unwrap_or(20),
        stat_gap: get_value(&css, ".stat-pair", "margin-right")
            .map(|value| strip_px(&value))
            .unwrap_or(12),
        tray_icon_size: get_value(&css, ".tray-btn image", "-gtk-icon-size")
            .map(|value| strip_px(&value))
            .unwrap_or(16),
        notification_border_radius: get_value(&css, "window.breadbar-notification", "border-radius")
            .map(|value| strip_px(&value))
            .unwrap_or(6),
    }
}

pub fn save(style: &Style) -> Result<bool, String> {
    let mut css = get_css();
    let px = |number: u32| format!("{number}px");
    let edits: [(&str, &str, String); 9] = [
        (
            "*",
            "font-family",
            format!("'{}', sans-serif", style.font_family),
        ),
        ("*", "font-size", px(style.font_size)),
        (
            "window.breadbar",
            "border-radius",
            px(style.bar_border_radius),
        ),
        ("window.breadbar", "padding", px(style.bar_padding)),
        (
            ".workspace-btn",
            "opacity",
            format!("{:.2}", style.workspace_inactive_opacity),
        ),
        (".workspace-btn", "font-size", px(style.workspace_font_size)),
        (".stat-pair", "margin-right", px(style.stat_gap)),
        (".tray-btn image", "-gtk-icon-size", px(style.tray_icon_size)),
        (
            "window.breadbar-notification",
            "border-radius",
            px(style.notification_border_radius),
        ),
    ];
    for (selector, property, value) in &edits {
        if let Some(updated) = set_value(&css, selector, property, value) {
            css = updated;
        }
    }
    config::atomic_write(&css_path(), &css).map_err(|error| error.to_string())?;
    reload()
}

fn reload() -> Result<bool, String> {
    Ok(std::process::Command::new("pkill")
        .args(["-HUP", "-x", "breadbar"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_value_changes_and_the_rest_survives() {
        let css = "/* keep */\nwindow.breadbar {\n  padding: 4px;\n  margin: 2px;\n}\n";
        let updated = set_value(css, "window.breadbar", "padding", "8px").unwrap();
        assert!(updated.contains("padding: 8px;"));
        assert!(updated.contains("margin: 2px;"));
        assert!(updated.contains("/* keep */"));
        assert_eq!(
            get_value(&updated, "window.breadbar", "padding").as_deref(),
            Some("8px")
        );
    }
}
