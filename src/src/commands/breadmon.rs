//! breadmon is a TUI for live Hyprland monitor layout, mirroring, and
//! named profiles (`~/.config/breadmon/profiles/`). Display (this app)
//! edits `hypr/monitors.json` — the login-time layout Hyprland itself
//! reads. This module only launches the TUI; it does not write profiles.

#[tauri::command]
pub fn open_breadmon() {
    let _ = std::process::Command::new("breadmon").spawn();
}
