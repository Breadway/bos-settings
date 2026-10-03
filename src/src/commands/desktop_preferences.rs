//! Persist additional compositor controls with a managed Lua include.
//! The stock BOS settings loader remains intact, and custom JSON keys are retained.
use super::{config, util};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopPreferences {
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
impl Default for DesktopPreferences {
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
fn path() -> std::path::PathBuf {
    config::config_dir().join("hypr/bos-settings-v2.json")
}
#[tauri::command]
pub fn get_desktop_preferences() -> DesktopPreferences {
    let mut prefs: DesktopPreferences = std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if !path().exists() {
        if let Ok(s) = std::fs::read_to_string(config::config_dir().join("hypr/settings.json")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(n) = v.get("natural_scroll").and_then(|v| v.as_bool()) {
                    prefs.natural_scroll = n;
                }
            }
        }
    }
    prefs
}
fn validate(p: &DesktopPreferences) -> Result<(), String> {
    if !p.pointer_speed.is_finite()
        || !(-1.0..=1.0).contains(&p.pointer_speed)
        || !p.scroll_factor.is_finite()
        || !(0.1..=5.0).contains(&p.scroll_factor)
        || !["clickfinger", "buttonareas"].contains(&p.click_method.as_str())
        || ![24, 32, 48].contains(&p.cursor_size)
        || ![12, 14, 16, 18].contains(&p.text_size)
        || !["Varela Round", "Inter", "Noto Sans"].contains(&p.font_family.as_str())
    {
        return Err("Invalid desktop preferences".into());
    }
    Ok(())
}
fn lua(p: &DesktopPreferences) -> String {
    let click = p.click_method == "clickfinger";
    format!("-- Managed by BOS Settings.\nhl.config({{input={{sensitivity={speed},scroll_factor={scroll},touchpad={{scroll_factor={scroll},tap_to_click={tap},natural_scroll={natural},clickfinger_behavior={click},disable_while_typing={typing}}}}},animations={{enabled={motion}}}{border}}})\nhl.env('XCURSOR_SIZE', '{cursor}')\nhl.env('HYPRCURSOR_SIZE', '{cursor}')\nlocal theme = os.getenv('XCURSOR_THEME') or 'Bibata-Modern-Ice'\nif not theme:match('^[%w%._%-]+$') then theme = 'Bibata-Modern-Ice' end\nhl.dispatch(hl.dsp.exec_cmd('hyprctl setcursor ' .. theme .. ' {cursor}'))\n",speed=p.pointer_speed,scroll=p.scroll_factor,tap=p.tap_to_click,natural=p.natural_scroll,typing=p.disable_while_typing,motion=!p.reduce_motion,border=if p.stronger_borders{",general={border_size=4}"}else{""},cursor=p.cursor_size)
}
const HOOK: &str = "\n-- BOS Settings additional desktop preferences\npcall(dofile, (os.getenv('XDG_CONFIG_HOME') or (os.getenv('HOME') .. '/.config')) .. '/hypr/bos-settings-v2.lua')\n";
#[tauri::command]
pub fn save_desktop_preferences(preferences: DesktopPreferences) -> Result<(), String> {
    validate(&preferences)?;
    let main = config::config_dir().join("hypr/hyprland.lua");
    let mut main_text = std::fs::read_to_string(&main).map_err(|_| {
        "BOS Lua configuration was not found. Preferences have not been saved.".to_string()
    })?;
    if !main_text.contains("/hypr/bos-settings-v2.lua") {
        main_text.push_str(HOOK);
    }
    let mut stored = std::fs::read_to_string(path())
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let values = serde_json::to_value(&preferences).map_err(|e| e.to_string())?;
    let object = stored
        .as_object_mut()
        .ok_or("Desktop preferences must be a JSON object")?;
    for (key, value) in values.as_object().unwrap() {
        object.insert(key.clone(), value.clone());
    }
    config::atomic_write(
        &config::config_dir().join("hypr/bos-settings-v2.lua"),
        &lua(&preferences),
    )
    .map_err(|e| e.to_string())?;
    config::atomic_write(&main, &main_text).map_err(|e| e.to_string())?;
    config::atomic_write(
        &path(),
        &serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    save_gtk_size(&preferences)?;
    util::hypr_reload();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_lua_executes_against_typed_compositor_api() {
        use std::io::Write;
        for (motion, method) in [(false, "clickfinger"), (true, "buttonareas")] {
            let p = DesktopPreferences {
                reduce_motion: motion,
                click_method: method.into(),
                stronger_borders: true,
                ..Default::default()
            };
            let script = format!("hl={{config=function(c) assert(c.input.touchpad.clickfinger_behavior == {click}); assert(c.animations.enabled == {enabled}); assert(c.general.border_size == 4) end, env=function(k,v) assert(v == '24') end, dsp={{exec_cmd=function(s) assert(s:match('^hyprctl setcursor [%w%._%-]+ 24$')); return s end}}, dispatch=function(s) assert(type(s) == 'string') end}}\n{lua}",click=method == "clickfinger",enabled=!motion,lua=lua(&p));
            let mut child = std::process::Command::new("lua")
                .arg("-")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .spawn()
                .expect("Lua is required to check generated compositor fragments");
            child
                .stdin
                .take()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            assert!(child.wait().unwrap().success());
        }
    }

    #[test]
    fn reject_injection_and_invalid_values() {
        let mut p = DesktopPreferences {
            click_method: "'); reboot; --".into(),
            ..Default::default()
        };
        assert!(validate(&p).is_err());
        p = DesktopPreferences::default();
        p.pointer_speed = f64::NAN;
        assert!(validate(&p).is_err());
        p = DesktopPreferences::default();
        assert!(validate(&p).is_ok());
        assert!(lua(&p).contains("tap_to_click=true"));
    }
}

/// Only crate callers supply the filename; webview callers cannot write a Lua path.
pub(crate) fn install_lua_include(filename: &str, source: &str) -> Result<(), String> {
    let main = config::config_dir().join("hypr/hyprland.lua");
    let mut text = std::fs::read_to_string(&main)
        .map_err(|e| format!("Cannot update BOS Lua configuration: {e}"))?;
    let suffix = format!("/hypr/{filename}");
    if !text.contains(&suffix) {
        text.push_str(&format!("\n-- BOS Settings managed desktop settings\npcall(dofile, (os.getenv('XDG_CONFIG_HOME') or (os.getenv('HOME') .. '/.config')) .. '{suffix}')\n"));
    }
    config::atomic_write(&config::config_dir().join("hypr").join(filename), source)
        .map_err(|e| e.to_string())?;
    config::atomic_write(&main, &text).map_err(|e| e.to_string())
}

fn update_ini(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    let start = lines.iter().position(|l| l.trim() == "[Settings]");
    if let Some(start) = start {
        let end = (start + 1..lines.len())
            .find(|&i| lines[i].trim().starts_with('['))
            .unwrap_or(lines.len());
        if let Some(i) = (start + 1..end).find(|&i| {
            lines[i]
                .split_once('=')
                .is_some_and(|(k, _)| k.trim() == key)
        }) {
            lines[i] = format!("{key}={value}")
        } else {
            lines.insert(end, format!("{key}={value}"));
        }
    } else {
        lines.push("[Settings]".into());
        lines.push(format!("{key}={value}"));
    }
    lines.join("\n") + "\n"
}
fn save_gtk_size(p: &DesktopPreferences) -> Result<(), String> {
    for folder in ["gtk-3.0", "gtk-4.0"] {
        let path = config::config_dir().join(folder).join("settings.ini");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let text = update_ini(
            &text,
            "gtk-font-name",
            &format!("{} {}", p.font_family, p.text_size),
        );
        let text = update_ini(&text, "gtk-cursor-theme-size", &p.cursor_size.to_string());
        config::atomic_write(&path, &text).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[cfg(test)]
mod ini_tests {
    use super::*;
    #[test]
    fn keeps_other_sections_and_comments() {
        let s = update_ini(
            "# hi\n[Settings]\nfoo=bar\ngtk-font-name=Old 14\n[Other]\nx=1\n",
            "gtk-font-name",
            "Varela Round 16",
        );
        assert!(s.contains("# hi"));
        assert!(s.contains("foo=bar"));
        assert!(s.contains("[Other]\nx=1"));
        assert_eq!(s.matches("gtk-font-name=").count(), 1);
    }
}
