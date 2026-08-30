<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import HyprColorField from "$lib/components/HyprColorField.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import { debounce } from "$lib/debounce";

	// Common XKB layout codes. Not exhaustive  -  if the config holds
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
	const FOLLOW_MOUSE_MODES: { value: string; label: string }[] = [
		{ value: "0", label: "Click to focus" },
		{ value: "1", label: "Follow mouse" },
		{ value: "2", label: "Follow, keep focus on click" },
		{ value: "3", label: "Mouse and keys independent" },
	];

	const KB_LABELS: Record<string, string> = {
		us: "English (US)",
		gb: "English (UK)",
		de: "German",
		fr: "French",
		es: "Spanish",
		it: "Italian",
		pt: "Portuguese",
		nl: "Dutch",
		se: "Swedish",
		no: "Norwegian",
		dk: "Danish",
		fi: "Finnish",
		pl: "Polish",
		cz: "Czech",
		sk: "Slovak",
		hu: "Hungarian",
		ro: "Romanian",
		gr: "Greek",
		tr: "Turkish",
		ru: "Russian",
		ua: "Ukrainian",
		jp: "Japanese",
		kr: "Korean",
		cn: "Chinese",
		br: "Portuguese (Brazil)",
		ca: "English (Canada)",
		ch: "German (Switzerland)",
		be: "Belgian",
		at: "German (Austria)",
		ie: "English (Ireland)",
	};

	const LAYOUT_LABELS: Record<string, string> = {
		dwindle: "Tiling",
		master: "Master stack",
	};

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
	let loaded = $state(false);
	let followMouse = $state("1");

	let kbLayoutOptions = $derived(
		cfg && !COMMON_KB_LAYOUTS.includes(cfg.kb_layout) ? [...COMMON_KB_LAYOUTS, cfg.kb_layout] : COMMON_KB_LAYOUTS,
	);

	const persist = debounce(() => {
		if (!cfg) return;
		invoke("save_appearance", { appearance: { ...cfg, follow_mouse: Number(followMouse) } });
	}, 450);

	onMount(async () => {
		cfg = await invoke<Appearance>("get_appearance");
		followMouse = String(cfg.follow_mouse);
		loaded = true;
	});

	let primed = false;
	$effect(() => {
		if (!loaded || !cfg) return;
		JSON.stringify(cfg);
		void followMouse;
		if (!primed) {
			primed = true;
			return;
		}
		persist();
	});
</script>

<ViewScaffold title="Appearance">
	{#if cfg}
		<Group title="Windows">
			<SliderField label="Gaps between windows" bind:value={cfg.gaps_in} min={0} max={50} />
			<SliderField label="Gaps around the edge" bind:value={cfg.gaps_out} min={0} max={50} />
			<SliderField label="Border width" bind:value={cfg.border_size} min={0} max={10} />
			<HyprColorField label="Active border" bind:value={cfg.active_border} />
			<HyprColorField label="Inactive border" bind:value={cfg.inactive_border} />
			<SelectField label="Layout" bind:value={cfg.layout} options={["dwindle", "master"]} labels={LAYOUT_LABELS} />
			<SwitchField label="Resize by dragging the border" bind:value={cfg.resize_on_border} />
		</Group>

		<Group title="Effects">
			<SliderField label="Corner rounding" bind:value={cfg.rounding} min={0} max={30} />
			<SwitchField label="Blur" bind:value={cfg.blur_enabled} />
			{#if cfg.blur_enabled}
				<SliderField label="Blur size" bind:value={cfg.blur_size} min={0} max={20} />
				<SliderField label="Blur quality" bind:value={cfg.blur_passes} min={1} max={5} />
			{/if}
			<SwitchField label="Shadows" bind:value={cfg.shadow_enabled} />
			{#if cfg.shadow_enabled}
				<SliderField label="Shadow size" bind:value={cfg.shadow_range} min={0} max={40} />
			{/if}
		</Group>

		<Group title="Keyboard and mouse">
			<SelectField label="Keyboard layout" bind:value={cfg.kb_layout} options={kbLayoutOptions} labels={KB_LABELS} />
			<SelectField
				label="Focus"
				bind:value={followMouse}
				options={FOLLOW_MOUSE_MODES.map((m) => m.value)}
				labels={Object.fromEntries(FOLLOW_MOUSE_MODES.map((m) => [m.value, m.label]))}
			/>
			<SwitchField label="Natural scrolling" hint="Touchpad" bind:value={cfg.natural_scroll} />
		</Group>

		<Hint text="Applies as you change it." />
	{/if}
</ViewScaffold>
