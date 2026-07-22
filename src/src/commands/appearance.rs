//! hypr/settings.json — Hyprland gaps/borders/blur/shadow/input, read by
//! `scripts/ui/settings.lua` on the Hyprland side. No comments to preserve,
//! so it's a plain typed struct round-tripped whole.
//!
//! `Default` here must stay in sync with `scripts/ui/settings.lua`'s
//! `DEFAULTS` table on the Hyprland side — two different languages/
//! processes reading the same file, neither able to import the other's
//! defaults.

use serde::{Deserialize, Serialize};

use super::config;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    gaps_in: i64,
    gaps_out: i64,
    border_size: i64,
    active_border: String,
    inactive_border: String,
    layout: String,
    resize_on_border: bool,
    rounding: i64,
    blur_enabled: bool,
    blur_size: i64,
    blur_passes: i64,
    shadow_enabled: bool,
    shadow_range: i64,
    shadow_render_power: i64,
    kb_layout: String,
    follow_mouse: i64,
    natural_scroll: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            gaps_in: 5,
            gaps_out: 10,
            border_size: 2,
            active_border: "rgba(88c0d0ff)".to_string(),
            inactive_border: "rgba(4c566aff)".to_string(),
            layout: "dwindle".to_string(),
            resize_on_border: true,
            rounding: 8,
            blur_enabled: true,
            blur_size: 6,
            blur_passes: 2,
            shadow_enabled: true,
            shadow_range: 12,
            shadow_render_power: 3,
            kb_layout: "us".to_string(),
            follow_mouse: 1,
            natural_scroll: true,
        }
    }
}

fn config_path() -> std::path::PathBuf {
    config::config_dir().join("hypr/settings.json")
}

#[tauri::command]
pub fn get_appearance() -> Appearance {
    std::fs::read_to_string(config_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[tauri::command]
pub fn save_appearance(appearance: Appearance) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&appearance).map_err(|e| e.to_string())?;
    config::atomic_write(&config_path(), &json).map_err(|e| e.to_string())
}
