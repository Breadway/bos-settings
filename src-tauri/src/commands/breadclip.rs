//! breadclip has no config file to edit — this panel exists purely to make
//! its background daemon (breadclipd, via `service.rs`) and its
//! on-demand popup visible/controllable from Settings.

#[tauri::command]
pub fn open_breadclip() {
    let _ = std::process::Command::new("breadclip").spawn();
}
