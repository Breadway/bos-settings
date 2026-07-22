// Maps a sidebar page id to its view component. Pages not yet migrated
// fall back to Placeholder (see +page.svelte) — this map only lists pages
// that actually have a real Tauri-backed view.

import type { Component } from "svelte";
import About from "./About.svelte";
import Breadclip from "./Breadclip.svelte";
import Bread from "./Bread.svelte";
import Breadbar from "./Breadbar.svelte";
import Breadbox from "./Breadbox.svelte";
import Breadpad from "./Breadpad.svelte";
import Breadpaper from "./Breadpaper.svelte";
import Breadsearch from "./Breadsearch.svelte";
import Breadcrumbs from "./Breadcrumbs.svelte";
import Appearance from "./Appearance.svelte";
import Autostart from "./Autostart.svelte";
import Display from "./Display.svelte";
import Keybinds from "./Keybinds.svelte";
import Sound from "./Sound.svelte";
import DateTime from "./DateTime.svelte";
import Power from "./Power.svelte";
import Network from "./Network.svelte";
import Bluetooth from "./Bluetooth.svelte";
import Firewall from "./Firewall.svelte";
import Users from "./Users.svelte";
import Packages from "./Packages.svelte";
import Aur from "./Aur.svelte";
import Firmware from "./Firmware.svelte";
import Snapshots from "./Snapshots.svelte";

export const VIEWS: Record<string, Component> = {
	about: About,
	breadclip: Breadclip,
	bread: Bread,
	breadbar: Breadbar,
	breadbox: Breadbox,
	breadpad: Breadpad,
	breadpaper: Breadpaper,
	breadsearch: Breadsearch,
	breadcrumbs: Breadcrumbs,
	appearance: Appearance,
	autostart: Autostart,
	hyprland: Display,
	keybinds: Keybinds,
	sound: Sound,
	datetime: DateTime,
	power: Power,
	network: Network,
	bluetooth: Bluetooth,
	firewall: Firewall,
	users: Users,
	packages: Packages,
	aur: Aur,
	firmware: Firmware,
	snapshots: Snapshots,
};
