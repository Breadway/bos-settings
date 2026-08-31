import type { Component } from "svelte";
import AboutHub from "./AboutHub.svelte";
import AppearanceHub from "./AppearanceHub.svelte";
import AppsHub from "./AppsHub.svelte";
import Bluetooth from "./Bluetooth.svelte";
import DesktopHub from "./DesktopHub.svelte";
import DisplaysHub from "./DisplaysHub.svelte";
import Home from "./Home.svelte";
import InputHub from "./InputHub.svelte";
import NetworkHub from "./NetworkHub.svelte";
import Power from "./Power.svelte";
import PrivacyHub from "./PrivacyHub.svelte";
import Sound from "./Sound.svelte";
import SystemHub from "./SystemHub.svelte";

export const VIEWS: Record<string, Component> = {
	home: Home,
	network: NetworkHub,
	bluetooth: Bluetooth,
	displays: DisplaysHub,
	sound: Sound,
	power: Power,
	appearance: AppearanceHub,
	desktop: DesktopHub,
	input: InputHub,
	apps: AppsHub,
	privacy: PrivacyHub,
	system: SystemHub,
	about: AboutHub,
};
