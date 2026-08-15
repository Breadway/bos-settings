//! breadhelp is the onboarding / help center. This panel launches it; it
//! does not duplicate the help library, keybind tour, or troubleshoot
//! wizard. First-run autostart (`breadhelp --autostart` in
//! `hypr/autostart.json`) is toggled through the existing autostart
//! commands from the frontend.

#[tauri::command]
pub fn open_breadhelp() {
    let _ = std::process::Command::new("breadhelp").spawn();
}
