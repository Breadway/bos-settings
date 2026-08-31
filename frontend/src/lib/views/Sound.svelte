<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface SoundDevice {
		name: string;
		description: string;
		mute: boolean;
		percent: number;
	}

	interface DeviceSection {
		kind: "sinks" | "sources";
		title: string;
		devices: SoundDevice[];
		selected: number;
	}

	let output = $state<DeviceSection | null>(null);
	let input = $state<DeviceSection | null>(null);

	function pick(devices: SoundDevice[], defaultName: string | null): number {
		const i = devices.findIndex((d) => d.name === defaultName);
		return i >= 0 ? i : 0;
	}

	async function loadSection(kind: "sinks" | "sources", title: string): Promise<DeviceSection> {
		const section = await invoke<{ devices: SoundDevice[]; default_name: string | null }>("get_sound_section", {
			kind,
		});
		return { kind, title, devices: section.devices, selected: pick(section.devices, section.default_name) };
	}

	onMount(async () => {
		try {
			output = await loadSection("sinks", "Output");
			input = await loadSection("sources", "Input");
		} catch (e) {
			console.error(e);
			output = { kind: "sinks", title: "Output", devices: [], selected: 0 };
			input = { kind: "sources", title: "Input", devices: [], selected: 0 };
		}
	});

	function current(section: DeviceSection): SoundDevice | null {
		return section.devices[section.selected] ?? section.devices[0] ?? null;
	}

	async function selectDevice(section: DeviceSection, index: number) {
		const device = section.devices[index];
		if (!device) return;
		section.selected = index;
		await invoke("set_default_sound_device", { kind: section.kind, name: device.name });
	}

	async function setVolume(section: DeviceSection, percent: number) {
		const device = current(section);
		if (!device) return;
		device.percent = percent;
		await invoke("set_sound_volume", { kind: section.kind, name: device.name, percent: Math.round(percent) });
	}

	async function setMute(section: DeviceSection, mute: boolean) {
		const device = current(section);
		if (!device) return;
		device.mute = mute;
		await invoke("set_sound_mute", { kind: section.kind, name: device.name, mute });
	}
</script>

{#snippet sectionCard(section: DeviceSection)}
	<Group title={section.title}>
		{#if section.devices.length === 0}
			<Hint text="No devices found." />
		{:else}
			{#each section.devices as d, i (d.name + i)}
				<button type="button" class="dev" class:on={section.selected === i} onclick={() => selectDevice(section, i)}>
					<span class="dev-name">{d.description || d.name}</span>
					{#if section.selected === i}<span class="mark">In use</span>{/if}
				</button>
			{/each}
			{#if current(section)}
				<Row label="Volume">
					<input
						type="range"
						min="0"
						max="150"
						value={Math.round(current(section)!.percent)}
						oninput={(e) => setVolume(section, Number(e.currentTarget.value))}
					/>
					<span class="pct">{Math.round(current(section)!.percent)}%</span>
				</Row>
				<SwitchField
					label="Mute"
					bind:value={() => current(section)!.mute, (v) => setMute(section, v)}
				/>
			{/if}
		{/if}
	</Group>
{/snippet}

<ViewScaffold title="Sound" lede="Output, input, and volume.">
	{#if output}
		{@render sectionCard(output)}
	{/if}
	{#if input}
		{@render sectionCard(input)}
	{/if}

	<Group title="Advanced">
		<button class="secondary" onclick={() => invoke("open_mixer")}>Per-app volume</button>
	</Group>
</ViewScaffold>

<style>
	.dev {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		text-align: left;
		background: transparent;
		border: none;
		border-top: 1px solid var(--line, #ffffff12);
		padding: 10px 2px;
		color: inherit;
		cursor: pointer;
	}

	.dev:first-child {
		border-top: none;
		padding-top: 0;
	}

	.dev.on .dev-name {
		font-weight: 600;
	}

	.dev-name {
		flex: 1;
	}

	.mark {
		font-size: 11px;
		padding: 4px 8px;
		border-radius: 999px;
		background: var(--accent);
		color: var(--on-accent);
		font-weight: 600;
	}

	input[type="range"] {
		width: 180px;
		accent-color: var(--accent);
	}

	.pct {
		margin-left: 8px;
		font-size: 12px;
		opacity: 0.7;
		min-width: 4ch;
	}

	.secondary {
		background-color: var(--bg);
		color: var(--on-surface);
		border: none;
		border-radius: 10px;
		padding: 8px 16px;
		cursor: pointer;
		align-self: flex-start;
	}
</style>
