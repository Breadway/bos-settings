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

	async function loadSection(kind: "sinks" | "sources", title: string): Promise<DeviceSection> {
		const section = await invoke<{ devices: SoundDevice[]; default_name: string | null }>("get_sound_section", { kind });
		const selected = Math.max(0, section.devices.findIndex((d) => d.name === section.default_name));
		return { kind, title, devices: section.devices, selected };
	}

	onMount(async () => {
		output = await loadSection("sinks", "Output");
		input = await loadSection("sources", "Input");
	});

	async function selectDevice(section: DeviceSection, index: number) {
		section.selected = index;
		await invoke("set_default_sound_device", { kind: section.kind, name: section.devices[index].name });
	}

	async function setVolume(section: DeviceSection, percent: number) {
		const device = section.devices[section.selected];
		device.percent = percent;
		await invoke("set_sound_volume", { kind: section.kind, name: device.name, percent });
	}

	async function setMute(section: DeviceSection, mute: boolean) {
		const device = section.devices[section.selected];
		device.mute = mute;
		await invoke("set_sound_mute", { kind: section.kind, name: device.name, mute });
	}
</script>

{#snippet deviceSection(section: DeviceSection | null)}
	{#if section}
		<Group title={section.title}>
			{#if section.devices.length === 0}
				<Hint text="No devices found." />
			{:else}
				<Row label="Device">
					<select
						value={section.selected}
						onchange={(e) => selectDevice(section, Number(e.currentTarget.value))}
					>
						{#each section.devices as d, i (d.name)}
							<option value={i}>{d.description}</option>
						{/each}
					</select>
				</Row>
				<Row label="Volume">
					<input
						type="range"
						min="0"
						max="150"
						value={section.devices[section.selected].percent}
						oninput={(e) => setVolume(section, Number(e.currentTarget.value))}
					/>
					<span class="pct">{section.devices[section.selected].percent}%</span>
				</Row>
				<SwitchField
					label="Mute"
					bind:value={
						() => section.devices[section.selected].mute,
						(v) => setMute(section, v)
					}
				/>
			{/if}
		</Group>
	{/if}
{/snippet}

<ViewScaffold title="Sound">
	{@render deviceSection(output)}
	{@render deviceSection(input)}

	<Group title="Advanced" hint="Per-app volume, port selection, and profile switching aren't covered here.">
		<button class="secondary" onclick={() => invoke("open_mixer")}>Open advanced mixer (pavucontrol)</button>
	</Group>
</ViewScaffold>

<style>
	select {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input[type="range"] {
		width: 180px;
	}

	.pct {
		margin-left: var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.7;
	}

	.secondary {
		background-color: var(--bg);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
		align-self: flex-start;
	}
</style>
