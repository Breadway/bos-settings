<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { createLiveGate } from "$lib/liveGate";
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
		virtual_device?: boolean;
	}

	interface AppStream {
		index: number;
		app: string;
		title: string;
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
	let streams = $state<AppStream[]>([]);

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

	// Volume, mute and the default device change behind this page's back (volume
	// keys, the bar, another mixer), so the backend streams `pactl subscribe`
	// events as `sound-changed` and the page re-reads on each one. A read never
	// replaces a slider the user is dragging or has just moved (BSET-31).
	const gate = createLiveGate();
	let retry: ReturnType<typeof setTimeout> | undefined;

	async function reload() {
		try {
			const [o, i] = [await loadSection("sinks", "Output"), await loadSection("sources", "Input")];
			if (!gate.canAdopt()) {
				clearTimeout(retry);
				retry = setTimeout(reload, 800);
				return;
			}
			output = o;
			input = i;
			streams = await invoke<AppStream[]>("get_app_streams");
		} catch (e) {
			console.error(e);
			output ??= { kind: "sinks", title: "Output", devices: [], selected: 0 };
			input ??= { kind: "sources", title: "Input", devices: [], selected: 0 };
		}
	}

	onMount(() => {
		reload();
		invoke("watch_sound");
		const unlisten = listen("sound-changed", reload);
		return () => {
			clearTimeout(retry);
			unlisten.then((f) => f());
		};
	});

	async function setStreamVolume(stream: AppStream, percent: number) {
		gate.touch();
		stream.percent = percent;
		await invoke("set_app_stream_volume", { index: stream.index, percent: Math.round(percent) }).catch(() => reload());
	}

	async function setStreamMute(stream: AppStream, mute: boolean) {
		gate.touch();
		stream.mute = mute;
		await invoke("set_app_stream_mute", { index: stream.index, mute }).catch(() => reload());
	}

	// Real speakers and microphones first; monitor sources and loopback devices
	// are listed under a disclosure unless one of them is the one in use.
	const shown = (section: DeviceSection, i: number, d: SoundDevice) => !d.virtual_device || section.selected === i;
	const hiddenCount = (section: DeviceSection) => section.devices.filter((d, i) => !shown(section, i, d)).length;

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
		gate.touch();
		device.percent = percent;
		await invoke("set_sound_volume", { kind: section.kind, name: device.name, percent: Math.round(percent) });
	}

	async function setMute(section: DeviceSection, mute: boolean) {
		const device = current(section);
		if (!device) return;
		gate.touch();
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
				{#if shown(section, i, d)}
				<button type="button" class="dev" class:on={section.selected === i} onclick={() => selectDevice(section, i)}>
					<span class="dev-name">{d.description || d.name}</span>
					{#if section.selected === i}<span class="mark">In use</span>{/if}
				</button>
				{/if}
			{/each}
			{#if hiddenCount(section)}
				<details class="more-devices"><summary>Monitor and loopback devices ({hiddenCount(section)})</summary>
					{#each section.devices as d, i (d.name + i)}
						{#if !shown(section, i, d)}
						<button type="button" class="dev" onclick={() => selectDevice(section, i)}><span class="dev-name">{d.description || d.name}</span></button>
						{/if}
					{/each}
				</details>
			{/if}
			{#if current(section)}
				<Row label="Volume">
					<input
						type="range"
						min="0"
						max="150"
						value={Math.round(current(section)!.percent)}
						onpointerdown={() => gate.begin()}
					onpointerup={() => gate.end()}
					onpointercancel={() => gate.end()}
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

	<Group title="Apps" hint={streams.length ? "Volume for each app that is playing sound right now." : "Apps that play sound show up here."}>
		{#each streams as stream (stream.index)}
			<Row label={stream.app} hint={stream.title}>
				<button class="secondary mute" aria-label={stream.mute ? `Unmute ${stream.app}` : `Mute ${stream.app}`} aria-pressed={stream.mute} onclick={() => setStreamMute(stream, !stream.mute)}>{stream.mute ? "Muted" : "Mute"}</button>
				<input type="range" min="0" max="150" value={Math.round(stream.percent)} aria-label={`${stream.app} volume`}
					onpointerdown={() => gate.begin()} onpointerup={() => gate.end()} onpointercancel={() => gate.end()}
					oninput={(e) => setStreamVolume(stream, Number(e.currentTarget.value))} />
				<span class="pct">{Math.round(stream.percent)}%</span>
			</Row>
		{/each}
		<button class="secondary" onclick={() => invoke("open_mixer")}>Open full mixer</button>
	</Group>
</ViewScaffold>

<style>
	.more-devices summary { cursor: pointer; color: var(--muted); font-size: 12.5px; padding: 8px 2px; }
	.mute { flex: none; }
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
