import type { Component } from "svelte";
import House from "@lucide/svelte/icons/house";
import Wifi from "@lucide/svelte/icons/wifi";
import Bluetooth from "@lucide/svelte/icons/bluetooth";
import Monitor from "@lucide/svelte/icons/monitor";
import Volume2 from "@lucide/svelte/icons/volume-2";
import BatteryFull from "@lucide/svelte/icons/battery-full";
import Palette from "@lucide/svelte/icons/palette";
import AppWindow from "@lucide/svelte/icons/app-window";
import Keyboard from "@lucide/svelte/icons/keyboard";
import LayoutGrid from "@lucide/svelte/icons/layout-grid";
import Shield from "@lucide/svelte/icons/shield";
import Download from "@lucide/svelte/icons/download";
import Info from "@lucide/svelte/icons/info";

export interface SidebarItem {
	id: string;
	label: string;
	icon: Component;
}

export interface SidebarSection {
	title: string | null;
	items: SidebarItem[];
}

export const SIDEBAR_SECTIONS: SidebarSection[] = [
	{ title: "Overview", items: [{ id: "home", label: "Home", icon: House }] },
	{
		title: "Devices",
		items: [
			{ id: "network", label: "Wi-Fi & internet", icon: Wifi },
			{ id: "bluetooth", label: "Bluetooth", icon: Bluetooth },
			{ id: "displays", label: "Displays", icon: Monitor },
			{ id: "sound", label: "Sound", icon: Volume2 },
			{ id: "power", label: "Power & battery", icon: BatteryFull },
		],
	},
	{
		title: "Desktop",
		items: [
			{ id: "appearance", label: "Appearance", icon: Palette },
			{ id: "desktop", label: "Bar & apps", icon: AppWindow },
			{ id: "input", label: "Keyboard & mouse", icon: Keyboard },
			{ id: "apps", label: "Default apps", icon: LayoutGrid },
		],
	},
	{
		title: "System",
		items: [
			{ id: "privacy", label: "Privacy & users", icon: Shield },
			{ id: "system", label: "Updates & backup", icon: Download },
			{ id: "about", label: "About", icon: Info },
		],
	},
];

/** Old sidebar ids (screenshot CLI, in-app jumps) → hub page + optional tab. */
export const PAGE_ALIASES: Record<string, { page: string; tab?: string }> = {
	home: { page: "home" },
	network: { page: "network" },
	vpn: { page: "network", tab: "vpn" },
	breadcrumbs: { page: "network", tab: "breadcrumbs" },
	bluetooth: { page: "bluetooth" },
	displays: { page: "displays" },
	hyprland: { page: "displays", tab: "hyprland" },
	nightlight: { page: "displays", tab: "nightlight" },
	breadmon: { page: "displays", tab: "breadmon" },
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

export const DEFAULT_PAGE = "home";

export function resolvePage(id: string): { page: string; tab?: string } {
	return PAGE_ALIASES[id] ?? { page: id };
}
