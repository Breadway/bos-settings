<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import HyprColorField from "$lib/components/HyprColorField.svelte";
	import Row from "$lib/components/Row.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	// Common XKB layout codes. Not exhaustive — if the config holds
	// something else, it's merged in below so it's never dropped from the
	// dropdown.
	const COMMON_KB_LAYOUTS = [
		"us", "gb", "de", "fr", "es", "it", "pt", "nl", "se", "no", "dk", "fi",
		"pl", "cz", "sk", "hu", "ro", "gr", "tr", "ru", "ua", "jp", "kr", "cn",
		"br", "ca", "ch", "be", "at", "ie",
	];

	// Hyprland's follow_mouse variable (see the wiki's Variables page):
	// 0 = cursor movement never changes focus; 1 = focus always follows the
	// window under the cursor; 2 = focus follows the cursor, but clicking a
	// window keeps keyboard focus there until the mouse moves again; 3 =
	// cursor and keyboard focus are fully independent.
	const FOLLOW_MOUSE_MODES: { value: number; label: string }[] = [
		{ value: 0, label: "Off — click to focus" },
		{ value: 1, label: "Follow mouse" },
		{ value: 2, label: "Follow mouse, detach on click" },
		{ value: 3, label: "Fully detached from keyboard focus" },
	];

	interface Appearance {
		gaps_in: number;
		gaps_out: number;
		border_size: number;
		active_border: string;
		inactive_border: string;
		layout: string;
		resize_on_border: boolean;
		rounding: number;
		blur_enabled: boolean;
		blur_size: number;
		blur_passes: number;
		shadow_enabled: boolean;
		shadow_range: number;
		shadow_render_power: number;
		kb_layout: string;
		follow_mouse: number;
		natural_scroll: boolean;
	}

	let cfg = $state<Appearance | null>(null);

	let kbLayoutOptions = $derived(
		cfg && !COMMON_KB_LAYOUTS.includes(cfg.kb_layout) ? [...COMMON_KB_LAYOUTS, cfg.kb_layout] : COMMON_KB_LAYOUTS,
	);

	onMount(async () => {
		cfg = await invoke<Appearance>("get_appearance");
	});

	async function save() {
		await invoke("save_appearance", { appearance: cfg });
	}
</script>

<ViewScaffold title="Appearance">
	{#if cfg}
		<Group title="Windows & borders" hint="Gaps, borders, and tiling — the same settings.json Hyprland reads at login.">
			<NumberField label="Gaps between windows" bind:value={cfg.gaps_in} min={0} max={50} />
			<NumberField label="Gaps around screen edge" bind:value={cfg.gaps_out} min={0} max={50} />
			<NumberField label="Border width" bind:value={cfg.border_size} min={0} max={10} />
			<HyprColorField label="Active border color" bind:value={cfg.active_border} />
			<HyprColorField label="Inactive border color" bind:value={cfg.inactive_border} />
			<SelectField label="Tiling layout" bind:value={cfg.layout} options={["dwindle", "master"]} />
			<SwitchField label="Resize by dragging borders" bind:value={cfg.resize_on_border} />
		</Group>

		<Group title="Effects">
			<NumberField label="Corner rounding" bind:value={cfg.rounding} min={0} max={30} />
			<SwitchField label="Blur" bind:value={cfg.blur_enabled} />
			<NumberField label="Blur size" bind:value={cfg.blur_size} min={0} max={20} />
			<NumberField label="Blur passes" bind:value={cfg.blur_passes} min={1} max={5} />
			<SwitchField label="Window shadows" bind:value={cfg.shadow_enabled} />
			<NumberField label="Shadow range" bind:value={cfg.shadow_range} min={0} max={40} />
			<NumberField label="Shadow render power" bind:value={cfg.shadow_render_power} min={1} max={4} />
		</Group>

		<Group title="Input">
			<SelectField label="Keyboard layout" bind:value={cfg.kb_layout} options={kbLayoutOptions} />
			<Row label="Focus-follows-mouse mode">
				<select bind:value={cfg.follow_mouse}>
					{#each FOLLOW_MOUSE_MODES as mode (mode.value)}
						<option value={mode.value}>{mode.label}</option>
					{/each}
				</select>
			</Row>
			<SwitchField label="Natural scrolling (touchpad)" bind:value={cfg.natural_scroll} />
		</Group>

		<Hint text="Changes apply on next login or Hyprland reload — this saves settings.json, it doesn't reload Hyprland live." />
		<SaveButton onSave={save} />
	{/if}
</ViewScaffold>

<style>
	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	option {
		background-color: var(--bg);
		color: var(--on-surface);
	}

	select:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
