mod commands;
mod screenshot;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let screenshot_req = screenshot::parse(&args);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            commands::theme::watch_and_emit(app.handle());
            if let Some(req) = screenshot_req {
                screenshot::dispatch(req, app.handle().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::theme::get_theme_css,
            commands::about::get_system_info,
            commands::about::set_hostname,
            commands::service::get_service_status,
            commands::service::service_action,
            commands::service::open_logs,
            commands::breadclip::open_breadclip,
            commands::bread::get_bread_config,
            commands::bread::save_bread_config,
            commands::bread::list_bread_modules,
            commands::breadpad::get_breadpad_config,
            commands::breadpad::save_breadpad_config,
            commands::breadsearch::get_breadsearch_config,
            commands::breadsearch::save_breadsearch_config,
            commands::breadbar::get_breadbar_css,
            commands::breadbar::save_breadbar_css,
            commands::breadbar::get_breadbar_style,
            commands::breadbar::save_breadbar_style,
            commands::breadbox::get_breadbox_contexts,
            commands::breadbox::save_breadbox_contexts,
            commands::breadcrumbs::get_breadcrumbs_config,
            commands::breadcrumbs::save_breadcrumbs_config,
            commands::breadpaper::get_current_wallpaper,
            commands::breadpaper::set_wallpaper,
            commands::breadpaper::list_wallpaper_library,
            commands::breadpaper::wallpaper_library_dir_display,
            commands::appearance::get_appearance,
            commands::appearance::save_appearance,
            commands::autostart::get_autostart_entries,
            commands::autostart::save_autostart_entries,
            commands::hyprland::get_live_monitors,
            commands::hyprland::get_monitor_rules,
            commands::hyprland::save_monitor_rules,
            commands::hyprland::open_hyprland_conf,
            commands::hyprland::open_keybinds_viewer,
            commands::keybinds::get_keybinds,
            commands::keybinds::save_keybinds,
            commands::sound::get_sound_section,
            commands::sound::set_default_sound_device,
            commands::sound::set_sound_volume,
            commands::sound::set_sound_mute,
            commands::sound::open_mixer,
            commands::datetime::get_datetime_info,
            commands::datetime::set_timezone,
            commands::datetime::set_ntp_enabled,
            commands::power::get_power_info,
            commands::power::set_brightness,
            commands::power::set_charge_threshold,
            commands::network::get_network_info,
            commands::network::set_wifi_radio,
            commands::network::scan_wifi,
            commands::network::connect_wifi,
            commands::network::open_connection_editor,
            commands::bluetooth::get_adapter_powered,
            commands::bluetooth::set_adapter_powered,
            commands::bluetooth::get_paired_devices,
            commands::bluetooth::scan_bluetooth,
            commands::bluetooth::bt_connect,
            commands::bluetooth::bt_disconnect,
            commands::bluetooth::bt_forget,
            commands::bluetooth::bt_pair,
            commands::firewall::get_firewall_status,
            commands::firewall::set_firewall_enabled,
            commands::firewall::add_firewall_rule,
            commands::firewall::remove_firewall_rule,
            commands::users::get_users_info,
            commands::users::change_password,
            commands::users::remove_user,
            commands::users::add_user,
            commands::streaming::bakery_update,
            commands::streaming::bakery_list,
            commands::streaming::bakery_update_all,
            commands::streaming::pacman_system_update,
            commands::streaming::fwupd_refresh,
            commands::streaming::fwupd_update,
            commands::packages::get_installed_packages,
            commands::aur::search_aur,
            commands::aur::install_aur_package,
            commands::firmware::get_updatable_firmware,
            commands::snapshots::get_snapshots,
            commands::snapshots::delete_snapshot,
            commands::snapshots::reboot_system,
            commands::breadlock::get_breadlock_config,
            commands::breadlock::save_breadlock_config,
            commands::breadlock::breadlock_example_path,
            commands::breadlock::open_breadlock_config,
            commands::breadlock::open_breadlock_example,
            commands::breadlock::lock_session,
            commands::breadshot::get_breadshot_config,
            commands::breadshot::save_breadshot_config,
            commands::breadshot::get_breadshot_binds,
            commands::breadshot::breadshot_region_clipboard,
            commands::breadmon::open_breadmon,
            commands::breadhelp::open_breadhelp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
