# Capability inventory before v2

Baseline: recovered feature/settings-v2 plus main, before the v2 layout changes. Existing backends and CLI flags must remain available.

## CLI and navigation

`--page <id>`, second-instance navigation, `--screenshot` and `screenshot-set-view` remain supported.

Legacy aliases:

```ts
: Record<string, { page: string; tab?: string }> = {
	home: { page: "home" },
	network: { page: "network" },
	vpn: { page: "network", tab: "vpn" },
	breadcrumbs: { page: "network", tab: "breadcrumbs" },
	bluetooth: { page: "bluetooth" },
	displays: { page: "displays" },
	hyprland: { page: "displays", tab: "hyprland" },
	nightlight: { page: "displays", tab: "nightlight" },
	breadmon: { page: "displays", tab: "hyprland" },
	sound: { page: "sound" },
	power: { page: "power" },
	appearance: { page: "appearance" },
	breadpaper: { page: "appearance", tab: "breadpaper" },
	desktop: { page: "desktop" },
	breadbar: { page: "desktop", tab: "breadbar" },
	breadbox: { page: "desktop", tab: "breadbox" },
	breadlock: { page: "desktop", tab: "breadlock" },
	breadshot: { page: "desktop", tab: "breadshot" },
	autostart: { page: "desktop", tab: "autostart" },
	breadclip: { page: "desktop", tab: "more" },
	breadpad: { page: "desktop", tab: "more" },
	breadsearch: { page: "desktop", tab: "more" },
	bread: { page: "desktop", tab: "more" },
	breadhelp: { page: "desktop", tab: "more" },
	more: { page: "desktop", tab: "more" },
	input: { page: "input" },
	keybinds: { page: "input", tab: "keybinds" },
	ime: { page: "input", tab: "ime" },
	accessibility: { page: "input", tab: "accessibility" },
	apps: { page: "apps" },
	defaults: { page: "apps" },
	optional: { page: "apps", tab: "optional" },
	printing: { page: "apps", tab: "printing" },
	privacy: { page: "privacy" },
	firewall: { page: "privacy" },
	users: { page: "privacy", tab: "users" },
	system: { page: "system" },
	updates: { page: "system" },
	packages: { page: "system", tab: "packages" },
	aur: { page: "system", tab: "aur" },
	firmware: { page: "system", tab: "firmware" },
	snapshots: { page: "system", tab: "snapshots" },
	backup: { page: "system", tab: "backup" },
	channel: { page: "system", tab: "channel" },
	about: { page: "about" },
	datetime: { page: "about", tab: "datetime" },
};
```

## Existing controls and commands

### About

Controls: About, This machine, Operating system, Kernel, CPU, GPU, Memory, Disk (/), Uptime, Hostname.

Commands: `get_system_info`, `set_hostname`.

### AboutHub

Controls: About.

Commands: .

### Accessibility

Controls: Accessibility, Screen reader, Orca, Magnifier, Zoom factor, Sticky keys / Slow keys, Sticky keys, Slow keys.

Commands: `pacman_install`, `set_orca_running`, `set_cursor_zoom`, `open_kmag`.

### Appearance

Controls: Appearance, Windows, Gaps between windows, Gaps around the edge, Border width, Active border, Inactive border, Layout, Resize by dragging the border, Effects, Corner rounding, Blur, Blur size, Blur quality, Shadows, Shadow size, Keyboard and mouse, Keyboard layout, Focus, Natural scrolling.

Commands: `save_appearance`, `get_appearance`.

### AppearanceHub

Controls: Appearance.

Commands: .

### AppsHub

Controls: Default apps.

Commands: .

### Aur

Controls: AUR, Search, No results.

Commands: `search_aur`, `install_aur_package`.

### Autostart

Controls: Startup Apps, Extra autostart apps, Enabled, Browse for program.

Commands: `get_autostart_entries`, `save_autostart_entries`.

### Backup

Controls: Backup, Repository, Local path, Or SFTP, Actions, Restore, Restore into, Snapshots, No snapshots loaded.

Commands: `get_backup_config`, `save_backup_config`, `pacman_install`, `list_restic_snapshots`.

### Bluetooth

Controls: Bluetooth, No Bluetooth adapter found, Adapter, Paired devices, No paired devices, Available devices, Not scanned yet, No new devices found.

Commands: `get_paired_devices`, `get_adapter_powered`, `set_adapter_powered`, `bt_forget`, `scan_bluetooth`, `bt_pair`.

### Bread

Controls: Daemon, Log level, Socket path, Lua, Entry point, Module path, Modules, Load built-in modules, Disabled modules, Adapters, Hyprland, udev (devices), udev subsystems, Power, Power poll interval (s), Network, Bluetooth, Events, Dedup window (ms), Notifications, Default timeout (ms), Default urgency, notify-send path.

Commands: `get_bread_config`, `list_bread_modules`, `save_bread_config`.

### Breadbar

Controls: Bar, Text, Font, Font size, Bar shape, Corner rounding, Inner padding, Workspaces, Number size, Inactive dimness, Spacing and icons, Gap between stats, Tray icon size, Notification rounding, Advanced.

Commands: `save_breadbar_style`, `get_breadbar_css`, `get_breadbar_style`, `save_breadbar_css`.

### Breadbox

Controls: Launcher, Contexts.

Commands: `get_breadbox_contexts`, `save_breadbox_contexts`.

### Breadclip

Controls: Clipboard, Clipboard history.

Commands: `open_breadclip`.

### Breadcrumbs

Controls: Wi-Fi Profiles, Settings, Default profile, DNS, Exit node, Ping host, Connectivity check URL, Connection timeout (s), Check connectivity every (s), Saved networks, Location profiles, Networks in this profile, Auto-switch here when nearby.

Commands: `get_breadcrumbs_config`, `save_breadcrumbs_config`.

### Breadhelp

Controls: Help, BOS Help, Show on login, Show help on login.

Commands: `get_autostart_entries`, `save_autostart_entries`, `open_breadhelp`.

### Breadlock

Controls: Lock, How locking works, Lock screen, Background, Image, Blur background, Clock format, Font, Wrong-password timeout (ms), Files.

Commands: `get_breadlock_config`, `breadlock_example_path`, `save_breadlock_config`, `lock_session`, `open_breadlock_config`, `open_breadlock_example`.

### Breadpad

Controls: Notes, Capture, Default note type, Tag with active workspace, Snooze options, Archive after (days), Classifier model, ONNX model, Tokenizer, Ollama (LLM classifier), Use Ollama, Endpoint, Model, Confidence threshold, Reminders, Default morning time, Missed grace (minutes), Calendar (CalDAV), Sync to calendar, CalDAV URL, Username, Password.

Commands: `get_breadpad_config`, `save_breadpad_config`.

### Breadpaper

Controls: Wallpaper, Use the same wallpaper on every monitor, Library.

Commands: `get_current_wallpaper`, `get_wallpapers_by_output`, `get_live_monitors`, `wallpaper_library_dir_display`, `list_wallpaper_library`, `set_wallpaper`, `set_wallpaper_on`.

### Breadsearch

Controls: File Search, Power, Enabled, Index while on battery, Model, Search acceleration, Max file size (MB), Search, Result limit, Snippet length, Index, Folders to index, Excluded folders, File extensions.

Commands: `get_breadsearch_config`, `save_breadsearch_config`.

### Breadshot

Controls: Screenshots, Keybinds, Capture, Save directory, Silent (no notifications), Freeze screen during select, Notification timeout (ms), Filename date format.

Commands: `get_breadshot_binds`, `get_breadshot_config`, `save_breadshot_config`, `breadshot_region_clipboard`.

### Channel

Controls: Bakery channel, Track.

Commands: `get_bakery_track`, `set_bakery_track`, `bakery_update_all`.

### DateTime

Controls: Date & Time, Current time, Right now, Timezone, Network time, Synchronize automatically.

Commands: `get_datetime_info`, `set_timezone`, `set_ntp_enabled`.

### Defaults

Controls: Default apps, Apps.

Commands: `get_default_apps`, `save_default_apps`.

### DesktopHub

Controls: Bar & apps.

Commands: .

### DesktopMore

Controls: .

Commands: .

### Display

Controls: Displays, Arrangement, Monitor arrangement canvas. Drag to reorder monitors., Resolution, Scale, Rotation, Adaptive sync, Position.

Commands: `get_live_monitors`, `apply_monitor_layout`, `persist_monitor_layout`.

### DisplaysHub

Controls: Displays.

Commands: .

### Firewall

Controls: Firewall, Firewall enabled, Add rule, Rules, Loading…, Couldn't read firewall status, No rules.

Commands: `get_firewall_status`, `set_firewall_enabled`, `remove_firewall_rule`, `add_firewall_rule`.

### Firmware

Controls: Firmware, Updatable devices, No updatable firmware devices found.

Commands: `get_updatable_firmware`, `fwupd_refresh`, `fwupd_update`.

### Home

Controls: .

Commands: .

### InputHub

Controls: Keyboard & mouse.

Commands: .

### InputMethod

Controls: Input method, fcitx5, Enable fcitx5, Packages.

Commands: `get_ime_status`, `set_ime_enabled`, `pacman_install`, `open_fcitx_config`.

### Keybinds

Controls: Keybinds, Shortcut already in use, Failed to load, Defaults, Default mods, Bindings, Layout, Active layout, Media & function keys (globals), Common (every layout), Add layout.

Commands: `get_keybinds`, `save_keybinds`.

### Network

Controls: Wi-Fi, Radio, Ethernet, This network, Nearby, Advanced.

Commands: `set_wifi_radio`, `scan_wifi`, `connect_wifi`, `open_connection_editor`.

### NetworkHub

Controls: Wi-Fi & internet.

Commands: .

### NightLight

Controls: Night light, When, When night light is on, From, To, Warmth.

Commands: `get_nightlight`, `set_nightlight`, `pacman_install`.

### Optional

Controls: Optional software, Curated extras.

Commands: `get_optional_software`, `enable_flathub`.

### Packages

Controls: Packages, Bread ecosystem (bakery), No bakery packages found, System packages (pacman).

Commands: `get_installed_packages`, `bakery_update`, `bakery_list`, `bakery_update_all`, `pacman_system_update`.

### Power

Controls: Power & battery, Battery, Power source, Brightness, Screen brightness, Battery protection, Protect battery, Charge limits, Start charging below, Stop charging at, TLP, Current profile.

Commands: `get_power_info`, `get_brightness`, `set_brightness`, `set_battery_protection`, `set_charge_threshold`.

### Printing

Controls: Printing, Printers, Couldn't talk to CUPS, No printers yet, IPP Everywhere, Name, URI.

Commands: `get_printers`, `set_default_printer`, `add_ipp_printer`, `open_printer_settings`.

### PrivacyHub

Controls: Privacy & users.

Commands: .

### ShellTheme

Controls: Shell theme, Theme, Apply.

Commands: `list_shell_themes`, `get_active_shell_theme`, `set_active_shell_theme`, `restart_shell_apps`.

### Snapshots

Controls: Snapshots, What to pick in GRUB, Couldn't read snapshots, No snapshots yet.

Commands: `get_snapshots`, `reboot_system`, `delete_snapshot`.

### Sound

Controls: Volume, Mute, Sound, Advanced.

Commands: `watch_sound`, `set_default_sound_device`, `set_sound_volume`, `set_sound_mute`, `open_mixer`.

### SystemHub

Controls: Updates & backup.

Commands: .

### Updates

Controls: Updates, NVIDIA driver, System packages (pacman), Pacman is up to date, Bread ecosystem (bakery), Bakery packages are current, Firmware (fwupd), No updatable firmware, Rollback.

Commands: `get_updates_status`.

### Users

Controls: Users, Accounts, Add user, Username, Full name, Password.

Commands: `change_password`, `remove_user`, `add_user`.

### Vpn

Controls: VPN / WireGuard, NetworkManager tunnels, Couldn't list connections, No VPN or WireGuard connections, Import, Config.

Commands: `get_vpn_connections`, `vpn_import`.

## Regression review

Navigation and any Advanced disclosures are recorded in docs/v2-progress.md. No capability removal is authorised by this inventory.
