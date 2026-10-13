import PowerMenu from "./PowerMenu.svelte";
import type { Component } from "svelte";
import About from "./About.svelte";
import AppearanceHub from "./AppearanceHub.svelte";
import Bluetooth from "./Bluetooth.svelte";
import DesktopHub from "./DesktopHub.svelte";
import DisplaysHub from "./DisplaysHub.svelte";
import Home from "./Home.svelte";
import NetworkHub from "./NetworkHub.svelte";
import Power from "./Power.svelte";
import PrivacyHub from "./PrivacyHub.svelte";
import Sound from "./Sound.svelte";
import KeyboardHub from "./KeyboardHub.svelte";
import Mouse from "./Mouse.svelte";
import Printing from "./Printing.svelte";
import Breadlock from "./Breadlock.svelte";
import Defaults from "./Defaults.svelte";
import GetAppsHub from "./GetAppsHub.svelte";
import Autostart from "./Autostart.svelte";
import UpdatesHub from "./UpdatesHub.svelte";
import RestoreHub from "./RestoreHub.svelte";
import Users from "./Users.svelte";
import DateTime from "./DateTime.svelte";
import Accessibility from "./Accessibility.svelte";
import Storage from "./Storage.svelte";
import Sharing from "./Sharing.svelte";
import Notifications from "./Notifications.svelte";

export const VIEWS: Record<string, Component> = {
	sharing: Sharing,
	notifications: Notifications,
	storage: Storage,
	home: Home,
    "power-menu": PowerMenu,
	keyboard: KeyboardHub,
	mouse: Mouse,
	printers: Printing,
	lock: Breadlock,
	defaults: Defaults,
	getapps: GetAppsHub,
	startup: Autostart,
	updates: UpdatesHub,
	restore: RestoreHub,
	users: Users,
	datetime: DateTime,
	a11y: Accessibility,

	network: NetworkHub,
	bluetooth: Bluetooth,
	displays: DisplaysHub,
	sound: Sound,
	power: Power,
	appearance: AppearanceHub,
	desktop: DesktopHub,
	privacy: PrivacyHub,
	about: About,
};
