// Ports src/ui/sidebar.rs's declarative item lists verbatim. Grouped by task,
// not "app vs system internals" — a user thinks "I want to change my Wi-Fi",
// not "which of these is a bread-ecosystem app" (why breadcrumbs/Wi-Fi
// Profiles lives in System, not Personalization).

import type { Component } from "svelte";
import Wifi from "@lucide/svelte/icons/wifi";
import Network from "@lucide/svelte/icons/network";
import Bluetooth from "@lucide/svelte/icons/bluetooth";
import Shield from "@lucide/svelte/icons/shield";
import Volume2 from "@lucide/svelte/icons/volume-2";
import BatteryFull from "@lucide/svelte/icons/battery-full";
import Clock from "@lucide/svelte/icons/clock";
import Monitor from "@lucide/svelte/icons/monitor";
import Keyboard from "@lucide/svelte/icons/keyboard";
import Rocket from "@lucide/svelte/icons/rocket";
import Users from "@lucide/svelte/icons/users";
import Palette from "@lucide/svelte/icons/palette";
import Image from "@lucide/svelte/icons/image";
import LayoutGrid from "@lucide/svelte/icons/layout-grid";
import Grid3x3 from "@lucide/svelte/icons/grid-3x3";
import Clipboard from "@lucide/svelte/icons/clipboard";
import NotebookPen from "@lucide/svelte/icons/notebook-pen";
import Search from "@lucide/svelte/icons/search";
import Cog from "@lucide/svelte/icons/cog";
import Package from "@lucide/svelte/icons/package";
import RefreshCw from "@lucide/svelte/icons/refresh-cw";
import History from "@lucide/svelte/icons/history";
import Info from "@lucide/svelte/icons/info";
import Lock from "@lucide/svelte/icons/lock";
import Camera from "@lucide/svelte/icons/camera";
import AppWindow from "@lucide/svelte/icons/app-window";
import CircleHelp from "@lucide/svelte/icons/circle-help";

export interface SidebarItem {
	/** Must match a key in the view component map (see routing in +page.svelte). */
	id: string;
	label: string;
	/** Dim second line — the underlying binary/config name, for items whose
	 * human label doesn't already make that obvious. */
	sublabel?: string;
	icon: Component;
}

export const SYSTEM_ITEMS: SidebarItem[] = [
	{ id: "network", label: "Network", icon: Wifi },
	{ id: "breadcrumbs", label: "Wi-Fi Profiles", sublabel: "breadcrumbs", icon: Network },
	{ id: "bluetooth", label: "Bluetooth", icon: Bluetooth },
	{ id: "firewall", label: "Firewall", icon: Shield },
	{ id: "sound", label: "Sound", icon: Volume2 },
	{ id: "power", label: "Power", icon: BatteryFull },
	{ id: "datetime", label: "Date & Time", icon: Clock },
	{ id: "hyprland", label: "Display", sublabel: "monitors.json", icon: Monitor },
	{ id: "breadmon", label: "Monitors", sublabel: "breadmon", icon: AppWindow },
	{ id: "breadlock", label: "Lock & greet", sublabel: "breadlock", icon: Lock },
	{ id: "keybinds", label: "Keybinds", sublabel: "binds.json", icon: Keyboard },
	{ id: "breadshot", label: "Screenshots", sublabel: "breadshot", icon: Camera },
	{ id: "autostart", label: "Startup Apps", sublabel: "autostart.json", icon: Rocket },
	{ id: "users", label: "Users", icon: Users },
];

export const PERSONALIZATION_ITEMS: SidebarItem[] = [
	{ id: "appearance", label: "Appearance", sublabel: "settings.json", icon: Palette },
	{ id: "breadpaper", label: "Wallpaper", sublabel: "breadpaper", icon: Image },
	{ id: "breadbar", label: "Bar", sublabel: "breadbar", icon: LayoutGrid },
	{ id: "breadbox", label: "Launcher", sublabel: "breadbox", icon: Grid3x3 },
	{ id: "breadclip", label: "Clipboard", sublabel: "breadclipd", icon: Clipboard },
	{ id: "breadpad", label: "Notes", sublabel: "breadpad", icon: NotebookPen },
	{ id: "breadsearch", label: "File Search", sublabel: "breadsearch", icon: Search },
	{ id: "bread", label: "Daemon", sublabel: "breadd", icon: Cog },
];

export const MAINTENANCE_ITEMS: SidebarItem[] = [
	{ id: "packages", label: "Packages", icon: Package },
	{ id: "aur", label: "AUR", icon: Search },
	{ id: "firmware", label: "Firmware", icon: RefreshCw },
	{ id: "snapshots", label: "Snapshots", icon: History },
];

export const ABOUT_ITEMS: SidebarItem[] = [
	{ id: "breadhelp", label: "Help", sublabel: "breadhelp", icon: CircleHelp },
	{ id: "about", label: "About", icon: Info },
];

export interface SidebarSection {
	title: string | null;
	items: SidebarItem[];
}

export const SIDEBAR_SECTIONS: SidebarSection[] = [
	{ title: "System", items: SYSTEM_ITEMS },
	{ title: "Personalization", items: PERSONALIZATION_ITEMS },
	{ title: "Maintenance", items: MAINTENANCE_ITEMS },
	{ title: null, items: ABOUT_ITEMS },
];

export const DEFAULT_PAGE = "about";
