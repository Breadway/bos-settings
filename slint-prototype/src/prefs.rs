//! hypr/bos-settings-v2.json plus the managed Lua include, matching the
//! shipping desktop preferences backend. Saving updates the JSON, the include,
//! the hook in hyprland.lua, GTK font and cursor sizes, then reloads Hyprland.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub pointer_speed: f64,
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    pub scroll_factor: f64,
    pub click_method: String,
    pub disable_while_typing: bool,
    pub cursor_size: u32,
    pub text_size: u32,
    pub font_family: String,
    pub reduce_motion: bool,
    pub stronger_borders: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            pointer_speed: 0.0,
            tap_to_click: true,
            natural_scroll: true,
            scroll_factor: 1.0,
            click_method: "clickfinger".into(),
            disable_while_typing: true,
            cursor_size: 24,
            text_size: 14,
            font_family: "Varela Round".into(),
            reduce_motion: false,
            stronger_borders: false,
        }
    }
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

fn path() -> PathBuf {
    config_dir().join("hypr/bos-settings-v2.json")
}

pub fn load() -> Preferences {
    let mut prefs: Preferences = std::fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if !path().exists() {
        if let Ok(text) = std::fs::read_to_string(config_dir().join("hypr/settings.json")) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(natural) = value.get("natural_scroll").and_then(|v| v.as_bool()) {
                    prefs.natural_scroll = natural;
                }
            }
        }
    }
    prefs
}

fn validate(prefs: &Preferences) -> Result<(), String> {
    if !prefs.pointer_speed.is_finite()
        || !(-1.0..=1.0).contains(&prefs.pointer_speed)
        || !prefs.scroll_factor.is_finite()
        || !(0.1..=5.0).contains(&prefs.scroll_factor)
        || !["clickfinger", "buttonareas"].contains(&prefs.click_method.as_str())
        || ![24, 32, 48].contains(&prefs.cursor_size)
        || ![12, 14, 16, 18].contains(&prefs.text_size)
        || !["Varela Round", "Inter", "Noto Sans"].contains(&prefs.font_family.as_str())
    {
        return Err("invalid desktop preferences".into());
    }
    Ok(())
}

fn lua(prefs: &Preferences) -> String {
    let click = prefs.click_method == "clickfinger";
    let border = if prefs.stronger_borders {
        ",general={border_size=4}"
    } else {
        ""
    };
    format!(
        "-- Managed by BOS Settings.\n\
hl.config({{input={{sensitivity={speed},scroll_factor={scroll},\
touchpad={{scroll_factor={scroll},tap_to_click={tap},natural_scroll={natural},\
clickfinger_behavior={click},disable_while_typing={typing}}}}},\
animations={{enabled={motion}}}{border}}})\n\
hl.env('XCURSOR_SIZE', '{cursor}')\n\
hl.env('HYPRCURSOR_SIZE', '{cursor}')\n\
local theme = os.getenv('XCURSOR_THEME') or 'Bibata-Modern-Ice'\n\
if not theme:match('^[%w%._%-]+$') then theme = 'Bibata-Modern-Ice' end\n\
hl.dispatch(hl.dsp.exec_cmd('hyprctl setcursor ' .. theme .. ' {cursor}'))\n",
        speed = prefs.pointer_speed,
        scroll = prefs.scroll_factor,
        tap = prefs.tap_to_click,
        natural = prefs.natural_scroll,
        click = click,
        typing = prefs.disable_while_typing,
        motion = !prefs.reduce_motion,
        cursor = prefs.cursor_size,
    )
}

const HOOK: &str = "\n-- BOS Settings additional desktop preferences\npcall(dofile, (os.getenv('XDG_CONFIG_HOME') or (os.getenv('HOME') .. '/.config')) .. '/hypr/bos-settings-v2.lua')\n";

fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!("{} has no parent", path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let temp = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "settings".into()),
        std::process::id()
    ));
    std::fs::write(&temp, text).map_err(|error| format!("{}: {error}", temp.display()))?;
    if path.exists() {
        let backup = path.with_extension(format!(
            "{}.bak",
            path.extension()
                .map(|extension| extension.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        std::fs::copy(path, &backup)
            .map_err(|error| format!("{}: {error}", backup.display()))?;
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        format!("{}: {error}", path.display())
    })
}

fn update_ini(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if let Some(start) = lines.iter().position(|line| line.trim() == "[Settings]") {
        let end = (start + 1..lines.len())
            .find(|&index| lines[index].trim().starts_with('['))
            .unwrap_or(lines.len());
        if let Some(index) = (start + 1..end).find(|&index| {
            lines[index]
                .split_once('=')
                .is_some_and(|(name, _)| name.trim() == key)
        }) {
            lines[index] = format!("{key}={value}");
        } else {
            lines.insert(end, format!("{key}={value}"));
        }
    } else {
        lines.push("[Settings]".into());
        lines.push(format!("{key}={value}"));
    }
    lines.join("\n") + "\n"
}

fn save_gtk(prefs: &Preferences) -> Result<(), String> {
    for folder in ["gtk-3.0", "gtk-4.0"] {
        let path = config_dir().join(folder).join("settings.ini");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let text = update_ini(
            &text,
            "gtk-font-name",
            &format!("{} {}", prefs.font_family, prefs.text_size),
        );
        let text = update_ini(&text, "gtk-cursor-theme-size", &prefs.cursor_size.to_string());
        write_atomic(&path, &text)?;
    }
    Ok(())
}

pub fn save(prefs: &Preferences) -> Result<(), String> {
    save_to(prefs, &config_dir().join("hypr"))
}

pub fn save_to(prefs: &Preferences, hypr_dir: &Path) -> Result<(), String> {
    validate(prefs)?;
    let main = hypr_dir.join("hyprland.lua");
    let mut main_text = std::fs::read_to_string(&main).map_err(|_| {
        "Hyprland's config file (hyprland.lua) was not found. Preferences have not been saved."
            .to_string()
    })?;
    if !main_text.contains("/hypr/bos-settings-v2.lua") {
        main_text.push_str(HOOK);
    }
    let stored_path = hypr_dir.join("bos-settings-v2.json");
    let mut stored = std::fs::read_to_string(&stored_path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let values = serde_json::to_value(prefs).map_err(|error| error.to_string())?;
    let object = stored
        .as_object_mut()
        .ok_or("desktop preferences must be a JSON object")?;
    for (key, value) in values.as_object().expect("struct serializes to an object") {
        object.insert(key.clone(), value.clone());
    }
    write_atomic(&hypr_dir.join("bos-settings-v2.lua"), &lua(prefs))?;
    write_atomic(&main, &main_text)?;
    write_atomic(
        &stored_path,
        &serde_json::to_string_pretty(&stored).map_err(|error| error.to_string())?,
    )?;
    save_gtk(prefs)?;
    let _ = std::process::Command::new("hyprctl").arg("reload").status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_states_every_managed_value() {
        let mut prefs = Preferences::default();
        prefs.pointer_speed = 0.3;
        prefs.scroll_factor = 1.5;
        prefs.click_method = "buttonareas".into();
        prefs.reduce_motion = true;
        prefs.stronger_borders = true;
        let text = lua(&prefs);
        assert!(text.contains("sensitivity=0.3"));
        assert!(text.contains("scroll_factor=1.5"));
        assert!(text.contains("clickfinger_behavior=false"));
        assert!(text.contains("animations={enabled=false}"));
        assert!(text.contains("border_size=4"));
        assert!(text.contains("XCURSOR_SIZE"));
    }

    #[test]
    fn invalid_values_are_refused() {
        let mut prefs = Preferences::default();
        prefs.pointer_speed = 4.0;
        assert!(validate(&prefs).is_err());
        prefs.pointer_speed = 0.0;
        prefs.cursor_size = 20;
        assert!(validate(&prefs).is_err());
    }

    #[test]
    fn ini_keeps_other_settings() {
        let text = update_ini(
            "# hi\n[Settings]\nfoo=bar\ngtk-font-name=Old 14\n[Other]\nx=1\n",
            "gtk-font-name",
            "Varela Round 16",
        );
        assert!(text.contains("# hi"));
        assert!(text.contains("foo=bar"));
        assert!(text.contains("[Other]\nx=1"));
        assert_eq!(text.matches("gtk-font-name=").count(), 1);
    }
}
