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

import Share2 from "@lucide/svelte/icons/share-2";

import Mouse from "@lucide/svelte/icons/mouse";

import Printer from "@lucide/svelte/icons/printer";

import Bell from "@lucide/svelte/icons/bell";

import Power from "@lucide/svelte/icons/power";

import History from "@lucide/svelte/icons/history";

import HardDrive from "@lucide/svelte/icons/hard-drive";

import Users from "@lucide/svelte/icons/users";

import Globe from "@lucide/svelte/icons/globe";

import Accessibility from "@lucide/svelte/icons/accessibility";

import Lock from "@lucide/svelte/icons/lock";

import Package from "@lucide/svelte/icons/package";

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
	{ title: null, items: [
		{ id: "home", label: "Home", icon: House },
	] },
	{ title: "Connect", items: [
		{ id: "network", label: "Wi-Fi & network", icon: Wifi },
		{ id: "bluetooth", label: "Bluetooth", icon: Bluetooth },
		{ id: "sharing", label: "Sharing", icon: Share2 },
	] },
	{ title: "Devices", items: [
		{ id: "displays", label: "Displays", icon: Monitor },
		{ id: "sound", label: "Sound", icon: Volume2 },
		{ id: "power", label: "Power & battery", icon: BatteryFull },
		{ id: "mouse", label: "Mouse & touchpad", icon: Mouse },
		{ id: "keyboard", label: "Keyboard", icon: Keyboard },
		{ id: "printers", label: "Printers", icon: Printer },
	] },
	{ title: "Personalise", items: [
		{ id: "appearance", label: "Appearance", icon: Palette },
		{ id: "desktop", label: "Desktop & bar", icon: AppWindow },
		{ id: "lock", label: "Lock screen", icon: Lock },
		{ id: "notifications", label: "Notifications", icon: Bell },
	] },
	{ title: "Apps", items: [
		{ id: "defaults", label: "Default apps", icon: LayoutGrid },
		{ id: "getapps", label: "Get apps", icon: Package },
		{ id: "startup", label: "Startup", icon: Power },
	] },
	{ title: "System", items: [
		{ id: "updates", label: "Updates", icon: Download },
		{ id: "restore", label: "Backup & restore", icon: History },
		{ id: "storage", label: "Storage", icon: HardDrive },
		{ id: "privacy", label: "Privacy & security", icon: Shield },
		{ id: "users", label: "Users", icon: Users },
		{ id: "datetime", label: "Date & language", icon: Globe },
		{ id: "a11y", label: "Accessibility", icon: Accessibility },
		{ id: "about", label: "About", icon: Info },
	] },
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
	breadmon: { page: "displays", tab: "hyprland" },
	sound: { page: "sound" },
	power: { page: "power" },
	appearance: { page: "appearance" },
	breadpaper: { page: "appearance", tab: "breadpaper" },
	desktop: { page: "desktop" },
	breadbar: { page: "desktop", tab: "breadbar" },
	breadbox: { page: "desktop", tab: "breadbox" },
	breadlock: { page: "lock" },
	breadshot: { page: "desktop", tab: "breadshot" },
	autostart: { page: "startup" },
	breadclip: { page: "desktop", tab: "breadclip" },
	breadpad: { page: "desktop", tab: "breadpad" },
	breadsearch: { page: "desktop", tab: "breadsearch" },
	bread: { page: "desktop", tab: "bread" },
	breadhelp: { page: "desktop", tab: "breadhelp" },
	more: { page: "desktop", tab: "breadclip" },
	input: { page: "keyboard", tab: "keybinds" },
	keybinds: { page: "keyboard", tab: "keybinds" },
	ime: { page: "keyboard", tab: "ime" },
	accessibility: { page: "a11y" },
	apps: { page: "defaults" },
	defaults: { page: "defaults" },
	optional: { page: "getapps", tab: "optional" },
	printing: { page: "printers" },
	privacy: { page: "privacy" },
	breadbug: { page: "privacy", tab: "bugs" },
	bugreports: { page: "privacy", tab: "bugs" },
	firewall: { page: "privacy" },
	users: { page: "users" },
	system: { page: "updates" },
	updates: { page: "updates" },
	packages: { page: "getapps", tab: "packages" },
	aur: { page: "getapps", tab: "aur" },
	firmware: { page: "updates", tab: "firmware" },
	snapshots: { page: "restore", tab: "snapshots" },
	backup: { page: "restore", tab: "backup" },
	channel: { page: "updates", tab: "channel" },
	about: { page: "about" },
	datetime: { page: "datetime" },
};

export const DEFAULT_PAGE = "home";

export function resolvePage(id: string): { page: string; tab?: string } {
	return PAGE_ALIASES[id] ?? { page: id };
}

/** Preserve old hub + tab requests while routing to the new sidebar pages. */
export function resolveDestination(page: string, tab?: string): {page:string; tab?:string} {
 const splitTabs: Record<string,string[]> = {
  desktop: ["breadlock", "autostart"],
  input: ["keybinds", "ime", "accessibility"],
  apps: ["printing", "optional"],
  privacy: ["users"],
  system: ["updates", "packages", "aur", "firmware", "snapshots", "backup", "channel"],
  about: ["datetime"],
 };
 if (page === "desktop" && tab === "more") return {page:"desktop",tab:"breadclip"};
 if (tab && splitTabs[page]?.includes(tab)) return resolvePage(tab);
 const resolved = resolvePage(page);
 return {page:resolved.page, tab:tab ?? resolved.tab};
}
