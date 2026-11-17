# Port status

Views: 63. Command functions: 285.

Status: [ ] todo, [~] partial, [x] done. A [x] needs a working port in this tree.

parity_check.py fails while any item is not [x].

## Home
- [ ] command `dashboard::get_current_account`
- [ ] command `dashboard::get_radio_states`
- [ ] command `dashboard::restore_radio_states`
- [ ] command `dashboard::set_airplane_mode`
- [~] view `Home`

## Appearance
- [x] command `appearance::get_appearance` -> slint-prototype/src/data.rs
- [x] command `appearance::save_appearance` -> slint-prototype/src/data.rs
- [x] command `shell_theme::get_active_shell_theme` -> slint-prototype/src/data.rs
- [x] command `shell_theme::list_shell_themes` -> slint-prototype/src/data.rs
- [x] command `shell_theme::restart_shell_apps` -> slint-prototype/src/data.rs
- [x] command `shell_theme::set_active_shell_theme` -> slint-prototype/src/data.rs
- [x] command `theme::get_theme_css` -> slint-prototype/src/theme.rs
- [x] command `theme::watch_and_emit` -> slint-prototype/src/theme.rs
- [x] view `Appearance` -> slint-prototype/ui/appearance.slint
- [x] view `AppearanceHub` -> slint-prototype/ui/appearance.slint
- [x] view `ShellTheme` -> slint-prototype/ui/appearance.slint
- [x] view `TextAppearance` -> slint-prototype/ui/appearance.slint
- [x] view `Windows` -> slint-prototype/ui/appearance.slint

## Network
- [x] command `network::connect_wifi` -> slint-prototype/src/network.rs
- [x] command `network::get_network_info` -> slint-prototype/src/network.rs
- [x] command `network::open_connection_editor` -> slint-prototype/src/network.rs
- [x] command `network::scan_wifi` -> slint-prototype/src/network.rs
- [x] command `network::set_wifi_radio` -> slint-prototype/src/network.rs
- [x] command `vpn::get_vpn_connections` -> slint-prototype/src/network.rs
- [x] command `vpn::vpn_connect` -> slint-prototype/src/network.rs
- [x] command `vpn::vpn_disconnect` -> slint-prototype/src/network.rs
- [x] command `vpn::vpn_import` -> slint-prototype/src/network.rs
- [x] view `Network` -> slint-prototype/src/pagedata.rs
- [x] view `NetworkAdvanced` -> slint-prototype/src/pagedata.rs
- [x] view `NetworkHub` -> slint-prototype/src/pagedata.rs
- [x] view `Places` -> slint-prototype/src/pagedata.rs
- [x] view `Vpn` -> slint-prototype/src/pagedata.rs

## Bluetooth
- [x] command `bluetooth::bt_connect` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::bt_disconnect` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::bt_forget` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::bt_pair` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::get_adapter_powered` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::get_bluetooth_state` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::get_paired_devices` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::scan_bluetooth` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::set_adapter_powered` -> slint-prototype/src/bluetooth.rs
- [x] command `bluetooth::start_bluetooth_service` -> slint-prototype/src/bluetooth.rs
- [x] view `Bluetooth` -> slint-prototype/src/pagedata.rs

## Sharing
- [x] command `sharing::get_sharing_status` -> slint-prototype/src/pagedata.rs
- [x] command `sharing::open_notification_history` -> slint-prototype/src/pagedata.rs
- [x] command `sharing::open_sharing_app` -> slint-prototype/src/pagedata.rs
- [x] command `sharing::set_remote_login` -> slint-prototype/src/pagedata.rs
- [~] view `Sharing`

## Displays
- [x] command `display_guard::apply` -> slint-prototype/src/displays.rs
- [x] command `display_guard::apply_display_layout` -> slint-prototype/src/displays.rs
- [x] command `display_guard::apply_for` -> slint-prototype/src/displays.rs
- [x] command `display_guard::disconnect_reason` -> slint-prototype/src/displays.rs
- [x] command `display_guard::display_guard_status` -> slint-prototype/src/displays.rs
- [x] command `display_guard::keep` -> slint-prototype/src/displays.rs
- [x] command `display_guard::keep_display_layout` -> slint-prototype/src/displays.rs
- [x] command `display_guard::layout_mismatch` -> slint-prototype/src/displays.rs
- [x] command `display_guard::new` -> slint-prototype/src/displays.rs
- [x] command `display_guard::plan_statements` -> slint-prototype/src/displays.rs
- [x] command `display_guard::production` -> slint-prototype/src/displays.rs
- [x] command `display_guard::revert` -> slint-prototype/src/displays.rs
- [x] command `display_guard::revert_display_layout` -> slint-prototype/src/displays.rs
- [x] command `display_guard::revert_on_exit` -> slint-prototype/src/displays.rs
- [ ] command `display_guard::revert_on_signal`
- [ ] command `display_guard::set_rescue`
- [x] command `display_guard::status` -> slint-prototype/src/displays.rs
- [x] command `display_guard::validate_layout` -> slint-prototype/src/displays.rs
- [ ] command `display_rescue::check_once`
- [ ] command `display_rescue::clear_record`
- [ ] command `display_rescue::decide`
- [ ] command `display_rescue::now_ms`
- [ ] command `display_rescue::owner_alive`
- [ ] command `display_rescue::read_record`
- [ ] command `display_rescue::record_path`
- [ ] command `display_rescue::recover_at_start`
- [ ] command `display_rescue::restore`
- [ ] command `display_rescue::run_watchdog`
- [ ] command `display_rescue::spawn_watchdog`
- [ ] command `display_rescue::write_record`
- [x] command `hyprland::get_live_monitors` -> slint-prototype/src/displays.rs
- [ ] command `hyprland::get_monitor_rules`
- [x] command `hyprland::open_hyprland_conf` -> slint-prototype/src/pagedata.rs
- [x] command `hyprland::open_keybinds_viewer` -> slint-prototype/src/pagedata.rs
- [ ] command `hyprland::save_monitor_rules`
- [ ] command `nightlight::get_nightlight`
- [ ] command `nightlight::set_nightlight`
- [ ] command `nvidia::get_nvidia_offer`
- [ ] command `nvidia::nvidia_setup`
- [ ] command `nvidia::read_nvidia_offer`
- [~] view `Display`
- [x] view `DisplaysHub` -> slint-prototype/src/pagedata.rs
- [~] view `NightLight`

## Sound
- [x] command `sound::get_app_streams` -> slint-prototype/src/sound.rs
- [x] command `sound::get_sound_section` -> slint-prototype/src/sound.rs
- [x] command `sound::open_mixer` -> slint-prototype/src/sound.rs
- [x] command `sound::set_app_stream_mute` -> slint-prototype/src/sound.rs
- [x] command `sound::set_app_stream_volume` -> slint-prototype/src/sound.rs
- [x] command `sound::set_default_sound_device` -> slint-prototype/src/sound.rs
- [x] command `sound::set_sound_mute` -> slint-prototype/src/sound.rs
- [x] command `sound::set_sound_volume` -> slint-prototype/src/sound.rs
- [ ] command `sound::watch_sound`
- [x] view `Sound` -> slint-prototype/src/pagedata.rs

## Power
- [x] command `power::get_battery_history` -> slint-prototype/src/pagedata.rs
- [x] command `power::get_brightness` -> slint-prototype/src/pagedata.rs
- [x] command `power::get_power_info` -> slint-prototype/src/pagedata.rs
- [ ] command `power::set_battery_protection`
- [x] command `power::set_brightness` -> slint-prototype/src/pagedata.rs
- [ ] command `power::set_charge_threshold`
- [ ] command `power_settings::enable_tlp_power_profiles`
- [ ] command `power_settings::get_automatic_power_mode`
- [x] command `power_settings::get_power_modes` -> slint-prototype/src/data.rs
- [ ] command `power_settings::get_power_settings`
- [ ] command `power_settings::run_power_action`
- [ ] command `power_settings::save_power_settings`
- [ ] command `power_settings::set_automatic_power_mode`
- [x] command `power_settings::set_power_mode` -> slint-prototype/src/data.rs
- [~] view `Power`
- [~] view `PowerMenu`

## Mouse
- [ ] command `desktop_preferences::get_desktop_preferences`
- [ ] command `desktop_preferences::save_desktop_preferences`
- [ ] view `Mouse`

## Keyboard
- [x] command `ime::get_ime_status` -> slint-prototype/src/ime.rs
- [x] command `ime::open_fcitx_config` -> slint-prototype/src/ime.rs
- [x] command `ime::set_ime_enabled` -> slint-prototype/src/ime.rs
- [x] command `keybinds::get_keybinds` -> slint-prototype/src/pagedata.rs
- [ ] command `keybinds::save_keybinds`
- [~] view `InputHub`
- [~] view `InputMethod`
- [~] view `Keybinds`
- [~] view `KeyboardHub`
- [x] view `KeyboardLayout` -> slint-prototype/src/pagedata.rs

## Printers
- [ ] command `printing::add_ipp_printer`
- [x] command `printing::get_printers` -> slint-prototype/src/pagedata.rs
- [x] command `printing::open_printer_settings` -> slint-prototype/src/pagedata.rs
- [x] command `printing::set_default_printer` -> slint-prototype/src/pagedata.rs
- [~] view `Printing`

## Lock screen
- [x] command `breadlock::breadlock_example_path` -> slint-prototype/src/lock.rs
- [x] command `breadlock::get_breadlock_config` -> slint-prototype/src/lock.rs
- [x] command `breadlock::lock_session` -> slint-prototype/src/lock.rs
- [x] command `breadlock::open_breadlock_config` -> slint-prototype/src/lock.rs
- [x] command `breadlock::open_breadlock_example` -> slint-prototype/src/lock.rs
- [x] command `breadlock::save_breadlock_config` -> slint-prototype/src/lock.rs
- [x] view `Breadlock` -> slint-prototype/src/pagedata.rs

## Notifications
- [x] command `notification_settings::get_notification_preferences` -> slint-prototype/src/notifications.rs
- [x] command `notification_settings::save_notification_preferences` -> slint-prototype/src/notifications.rs
- [x] view `Notifications` -> slint-prototype/src/pagedata.rs

## Default apps
- [x] command `defaults::get_default_apps` -> slint-prototype/src/pagedata.rs
- [ ] command `defaults::list_startup_apps`
- [x] command `defaults::save_default_apps` -> slint-prototype/src/pagedata.rs
- [x] view `Defaults` -> slint-prototype/src/pagedata.rs

## Get apps
- [x] command `app_store::install_store_app` -> slint-prototype/src/apps.rs
- [x] command `app_store::search_apps` -> slint-prototype/src/apps.rs
- [x] command `aur::install_aur_package` -> slint-prototype/src/apps.rs
- [x] command `aur::search_aur` -> slint-prototype/src/apps.rs
- [x] command `optional::enable_flathub` -> slint-prototype/src/apps.rs
- [x] command `optional::get_optional_software` -> slint-prototype/src/apps.rs
- [x] command `packages::get_installed_packages` -> slint-prototype/src/apps.rs
- [~] view `AppsHub`
- [~] view `Aur`
- [x] view `GetAppsHub` -> slint-prototype/src/pagedata.rs
- [~] view `Optional`
- [~] view `Packages`

## Startup
- [ ] command `autostart::command`
- [x] command `autostart::get_autostart_entries` -> slint-prototype/src/pagedata.rs
- [x] command `autostart::load_entries` -> slint-prototype/src/pagedata.rs
- [ ] command `autostart::new`
- [x] command `autostart::remove_entries` -> slint-prototype/src/pagedata.rs
- [x] command `autostart::save_autostart_entries` -> slint-prototype/src/pagedata.rs
- [x] command `autostart::store_entries` -> slint-prototype/src/pagedata.rs
- [x] command `autostart::upsert_entry` -> slint-prototype/src/pagedata.rs
- [~] view `Autostart`

## Updates
- [x] command `bread::get_bread_config` -> slint-prototype/src/bread.rs
- [x] command `bread::list_bread_modules` -> slint-prototype/src/bread.rs
- [x] command `bread::save_bread_config` -> slint-prototype/src/bread.rs
- [x] command `channel::get_bakery_track` -> slint-prototype/src/updates.rs
- [x] command `channel::set_bakery_track` -> slint-prototype/src/updates.rs
- [x] command `firmware::get_updatable_firmware` -> slint-prototype/src/updates.rs
- [x] command `updates::get_updates_status` -> slint-prototype/src/updates.rs
- [~] view `Bread`
- [ ] view `BugReports`
- [x] view `Channel` -> slint-prototype/src/pagedata.rs
- [~] view `Firmware`
- [x] view `Updates` -> slint-prototype/src/pagedata.rs
- [x] view `UpdatesHub` -> slint-prototype/src/pagedata.rs

## Backup and restore
- [x] command `backup::default_restore_dir` -> slint-prototype/src/backup.rs
- [x] command `backup::get_backup_config` -> slint-prototype/src/backup.rs
- [x] command `backup::get_backup_schedule` -> slint-prototype/src/backup.rs
- [x] command `backup::list_restic_snapshots` -> slint-prototype/src/backup.rs
- [x] command `backup::load_secrets` -> slint-prototype/src/backup.rs
- [x] command `backup::restic_backup` -> slint-prototype/src/backup.rs
- [x] command `backup::restic_init` -> slint-prototype/src/backup.rs
- [x] command `backup::restic_restore` -> slint-prototype/src/backup.rs
- [x] command `backup::restic_restore_dry_run` -> slint-prototype/src/backup.rs
- [x] command `backup::save_backup_config` -> slint-prototype/src/backup.rs
- [ ] command `backup::scheduled_backup`
- [x] command `backup::set_backup_schedule` -> slint-prototype/src/backup.rs
- [x] command `backup::valid_repo` -> slint-prototype/src/backup.rs
- [x] command `backup::valid_restore_target` -> slint-prototype/src/backup.rs
- [ ] command `snapshots::create_manual_snapshot`
- [ ] command `snapshots::create_update_snapshot`
- [x] command `snapshots::delete_snapshot` -> slint-prototype/src/backup.rs
- [x] command `snapshots::get_snapshot_availability` -> slint-prototype/src/backup.rs
- [x] command `snapshots::get_snapshots` -> slint-prototype/src/backup.rs
- [x] command `snapshots::reboot_system` -> slint-prototype/src/backup.rs
- [ ] command `snapshots::restore_snapshot`
- [~] view `Backup`
- [ ] view `RestoreHub`
- [~] view `Snapshots`

## Users
- [ ] command `users::add_user`
- [ ] command `users::change_password`
- [x] command `users::get_users_info` -> slint-prototype/src/pagedata.rs
- [ ] command `users::remove_user`
- [~] view `Users`

## Date and time
- [x] command `datetime::get_datetime_info` -> slint-prototype/src/pagedata.rs
- [x] command `datetime::set_ntp_enabled` -> slint-prototype/src/pagedata.rs
- [x] command `datetime::set_timezone` -> slint-prototype/src/pagedata.rs
- [ ] command `locale_settings::get_clock_format`
- [x] command `locale_settings::get_locale_info` -> slint-prototype/src/pagedata.rs
- [x] command `locale_settings::save_locale_choice` -> slint-prototype/src/pagedata.rs
- [x] command `locale_settings::set_clock_format` -> slint-prototype/src/pagedata.rs
- [x] view `DateTime` -> slint-prototype/src/pagedata.rs

## Accessibility
- [x] command `a11y::get_a11y_status` -> slint-prototype/src/pagedata.rs
- [x] command `a11y::open_kmag` -> slint-prototype/src/pagedata.rs
- [x] command `a11y::set_cursor_zoom` -> slint-prototype/src/pagedata.rs
- [x] command `a11y::set_orca_running` -> slint-prototype/src/pagedata.rs

## Storage
- [x] command `storage::clean_storage` -> slint-prototype/src/pagedata.rs
- [x] command `storage::get_cache_entries` -> slint-prototype/src/pagedata.rs
- [ ] command `storage::get_storage_breakdown`
- [x] command `storage::get_storage_info` -> slint-prototype/src/pagedata.rs
- [x] command `storage::get_unused_dependencies` -> slint-prototype/src/pagedata.rs
- [x] command `storage::open_storage_folder` -> slint-prototype/src/pagedata.rs
- [ ] command `storage::remove_unused_dependencies`
- [~] view `Storage`

## Memory
- [x] command `breadram::apply_breadram_profile` -> slint-prototype/src/breadram.rs
- [x] command `breadram::breadram_actions` -> slint-prototype/src/breadram.rs
- [x] command `breadram::breadram_installed` -> slint-prototype/src/breadram.rs
- [x] command `breadram::breadram_status` -> slint-prototype/src/breadram.rs
- [x] command `breadram::get_breadram_config` -> slint-prototype/src/breadram.rs
- [x] command `breadram::save_breadram_config` -> slint-prototype/src/breadram.rs
- [x] command `breadram::set_breadram_mode` -> slint-prototype/src/breadram.rs
- [x] command `breadram::set_breadram_protection` -> slint-prototype/src/breadram.rs
- [x] view `Memory` -> slint-prototype/src/pagedata.rs

## Desktop
- [ ] command `breadbar::get_breadbar_css`
- [ ] command `breadbar::get_breadbar_style`
- [ ] command `breadbar::save_breadbar_css`
- [ ] command `breadbar::save_breadbar_style`
- [ ] command `breadbox::get_breadbox_contexts`
- [ ] command `breadbox::save_breadbox_contexts`
- [x] command `breadclip::open_breadclip` -> slint-prototype/src/pagedata.rs
- [ ] command `breadhelp::open_breadhelp`
- [ ] command `breadpad::get_breadpad_config`
- [ ] command `breadpad::save_breadpad_config`
- [x] command `breadpaper::get_current_wallpaper` -> slint-prototype/src/data.rs
- [x] command `breadpaper::get_wallpapers_by_output` -> slint-prototype/src/data.rs
- [ ] command `breadpaper::list_wallpaper_library`
- [x] command `breadpaper::set_wallpaper` -> slint-prototype/src/data.rs
- [x] command `breadpaper::set_wallpaper_on` -> slint-prototype/src/data.rs
- [ ] command `breadpaper::wallpaper_library_dir`
- [ ] command `breadpaper::wallpaper_library_dir_display`
- [ ] command `breadsearch::get_breadsearch_config`
- [ ] command `breadsearch::save_breadsearch_config`
- [ ] command `breadshot::breadshot_region_clipboard`
- [ ] command `breadshot::get_breadshot_binds`
- [ ] command `breadshot::get_breadshot_config`
- [ ] command `breadshot::save_breadshot_config`
- [ ] view `Breadbar`
- [ ] view `Breadbox`
- [ ] view `Breadclip`
- [ ] view `Breadhelp`
- [ ] view `Breadpad`
- [x] view `Breadpaper` -> slint-prototype/ui/appearance.slint
- [ ] view `Breadsearch`
- [ ] view `Breadshot`
- [~] view `DesktopHub`

## Privacy and security
- [x] command `breadbug::apply_settings` -> slint-prototype/src/breadbug.rs
- [x] command `breadbug::breadbug_clear_queue` -> slint-prototype/src/breadbug.rs
- [ ] command `breadbug::breadbug_delete_report`
- [x] command `breadbug::breadbug_history` -> slint-prototype/src/breadbug.rs
- [x] command `breadbug::breadbug_wipe_local` -> slint-prototype/src/breadbug.rs
- [ ] command `breadbug::clean_exclusions`
- [x] command `breadbug::get_breadbug_settings` -> slint-prototype/src/breadbug.rs
- [x] command `breadbug::open_breadbug` -> slint-prototype/src/breadbug.rs
- [x] command `breadbug::save_breadbug_settings` -> slint-prototype/src/breadbug.rs
- [ ] command `firewall::add_firewall_rule`
- [x] command `firewall::get_firewall_service_active` -> slint-prototype/src/firewall.rs
- [x] command `firewall::get_firewall_status` -> slint-prototype/src/firewall.rs
- [ ] command `firewall::remove_firewall_allow`
- [ ] command `firewall::remove_firewall_rule`
- [x] command `firewall::set_firewall_enabled` -> slint-prototype/src/firewall.rs
- [~] view `Firewall`
- [~] view `PrivacyHub`

## About
- [x] command `about::get_system_info` -> slint-prototype/src/data.rs
- [x] command `about::run_health_checks` -> slint-prototype/src/data.rs
- [x] command `about::set_hostname` -> slint-prototype/src/data.rs
- [x] view `About` -> slint-prototype/ui/about.slint
- [x] view `AboutHub` -> slint-prototype/ui/about.slint

## System
- [ ] command `breadcrumbs::get_breadcrumbs_config`
- [ ] command `breadcrumbs::get_current_place`
- [ ] command `breadcrumbs::save_breadcrumbs_config`
- [ ] command `breadcrumbs::set_current_place`
- [x] command `config::atomic_write` -> slint-prototype/src/config.rs
- [x] command `config::config_dir` -> slint-prototype/src/config.rs
- [x] command `config::get_bool` -> slint-prototype/src/config.rs
- [x] command `config::get_f64` -> slint-prototype/src/config.rs
- [x] command `config::get_i64` -> slint-prototype/src/config.rs
- [x] command `config::get_str` -> slint-prototype/src/config.rs
- [x] command `config::get_str_list` -> slint-prototype/src/config.rs
- [x] command `config::get_str_list_strict` -> slint-prototype/src/config.rs
- [x] command `config::load_doc` -> slint-prototype/src/config.rs
- [x] command `config::remove` -> slint-prototype/src/config.rs
- [x] command `config::save_doc` -> slint-prototype/src/config.rs
- [x] command `config::set_bool` -> slint-prototype/src/config.rs
- [x] command `config::set_f64` -> slint-prototype/src/config.rs
- [x] command `config::set_i64` -> slint-prototype/src/config.rs
- [x] command `config::set_str` -> slint-prototype/src/config.rs
- [x] command `config::set_str_list` -> slint-prototype/src/config.rs
- [x] command `config::set_str_or_remove` -> slint-prototype/src/config.rs
- [ ] command `navigation::get_initial_page`
- [x] command `service::get_service_status` -> slint-prototype/src/pagedata.rs
- [ ] command `service::open_logs`
- [x] command `service::service_action` -> slint-prototype/src/pagedata.rs
- [ ] command `streaming::bakery_install`
- [ ] command `streaming::bakery_list`
- [ ] command `streaming::bakery_update`
- [ ] command `streaming::bakery_update_all`
- [ ] command `streaming::fwupd_refresh`
- [ ] command `streaming::fwupd_update`
- [ ] command `streaming::pacman_install`
- [ ] command `streaming::pacman_system_update`
- [ ] command `util::allowed_bakery_install`
- [ ] command `util::allowed_pacman_packages`
- [ ] command `util::bos_settings_dir`
- [ ] command `util::command_exists`
- [ ] command `util::ensure_hypr_source`
- [ ] command `util::fail_output`
- [ ] command `util::hypr_dir`
- [ ] command `util::hypr_reload`
- [ ] command `util::hyprland_conf`
- [ ] command `util::pacman_installed`
- [ ] command `util::remove_hypr_source`
- [ ] command `util::run_with_stdin`
- [ ] command `util::strip_ansi`
- [ ] command `util::valid_nm_id`
- [ ] command `util::valid_number_id`
- [ ] command `util::valid_pkg_name`
- [ ] command `util::valid_printer_name`
- [ ] command `util::write_secure`
- [ ] view `SystemHub`

## Other
- [ ] view `Accessibility`
