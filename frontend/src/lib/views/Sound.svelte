<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { createLiveGate } from "$lib/liveGate";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import Speaker from "@lucide/svelte/icons/speaker";
	import Headphones from "@lucide/svelte/icons/headphones";
	import Mic from "@lucide/svelte/icons/mic";
	import Monitor from "@lucide/svelte/icons/monitor";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import VolumeX from "@lucide/svelte/icons/volume-x";

	interface SoundDevice {
		name: string;
		description: string;
		mute: boolean;
		percent: number;
		virtual_device?: boolean;
	}

	// "Ryzen HD Audio Controller Speaker" -> Speakers · Built-in.
	function friendly(d: SoundDevice, kind: "sinks" | "sources"): { name: string; where: string; Icon: typeof Speaker } {
		const desc = d.description || d.name;
		const bt = d.name.startsWith("bluez");
		const hdmi = /hdmi|displayport/i.test(desc);
		const builtIn = /HD Audio Controller|Built-in|Analog|Speaker|Digital Microphone|sof-|acp/i.test(desc) && !bt && !hdmi;
		let name = desc;
		let Icon: typeof Speaker = kind === "sinks" ? Speaker : Mic;
		if (bt) { Icon = kind === "sinks" ? Headphones : Mic; }
		else if (hdmi) { name = desc.replace(/^.*?(HDMI|DisplayPort)/i, "$1"); Icon = Monitor; }
		else if (/headphone/i.test(desc)) { name = "Headphones"; Icon = Headphones; }
		else if (/speaker/i.test(desc)) name = "Speakers";
		else if (/digital microphone/i.test(desc)) name = "Microphone";
		else if (/stereo microphone|headset microphone|mic/i.test(desc) && builtIn) name = "Headset jack microphone";
		return { name, where: bt ? "Bluetooth" : hdmi ? "Screen" : builtIn ? "Built-in" : d.virtual_device ? "Virtual" : "External", Icon };
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

{#snippet card(section: DeviceSection, d: SoundDevice, i: number)}
	{@const f = friendly(d, section.kind)}
	<button type="button" class="dev-card pick" class:on={section.selected === i} onclick={() => selectDevice(section, i)} title={d.description || d.name}>
		<span class="t"><span class="ico"><f.Icon size={18} /></span><span><b>{f.name}</b><small>{f.where}{section.selected === i ? " · in use" : ""}</small></span></span>
	</button>
{/snippet}

{#snippet sectionCard(section: DeviceSection)}
	<Group title={section.title}>
		{#if section.devices.length === 0}
			<p class="muted">No devices found.</p>
		{:else}
			<div class="devgrid picks">
				{#each section.devices as d, i (d.name + i)}
					{#if shown(section, i, d)}{@render card(section, d, i)}{/if}
				{/each}
			</div>
			{#if hiddenCount(section)}
				<details class="ui-disclosure"><summary>Monitor and loopback devices ({hiddenCount(section)})</summary>
					<div class="devgrid picks">
						{#each section.devices as d, i (d.name + i)}
							{#if !shown(section, i, d)}{@render card(section, d, i)}{/if}
						{/each}
					</div>
				</details>
			{/if}
			{#if current(section)}
				{@const dev = current(section)!}
				<div class="vol">
					<button class="btn small ghost icon" aria-label={dev.mute ? "Unmute" : "Mute"} aria-pressed={dev.mute} onclick={() => setMute(section, !dev.mute)}>
						{#if dev.mute}<VolumeX size={16} />{:else if section.kind === "sources"}<Mic size={16} />{:else}<Volume2 size={16} />{/if}
					</button>
					<input type="range" min="0" max="150" value={Math.round(dev.percent)} aria-label="{section.title} volume"
						onpointerdown={() => gate.begin()} onpointerup={() => gate.end()} onpointercancel={() => gate.end()}
						oninput={(e) => setVolume(section, Number(e.currentTarget.value))} />
					<span class="pct">{dev.mute ? "Muted" : `${Math.round(dev.percent)}%`}</span>
				</div>
			{/if}
		{/if}
	</Group>
{/snippet}
<ViewScaffold title="Sound" lede="Where sound goes, what it picks up, and how loud each app is.">
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
		<div class="vol"><button class="btn small" onclick={() => invoke("open_mixer")}>Open full mixer</button></div>
	</Group>
</ViewScaffold>

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; }
	.picks { grid-template-columns:repeat(auto-fill, minmax(190px, 1fr)); padding:2px 0 10px; }
	.pick { cursor:pointer; color:inherit; font:inherit; text-align:left; padding:12px; }
	.pick:hover { border-color:color-mix(in oklab, var(--accent) 35%, var(--line)); }
	.pick .t > span:last-child { display:flex; flex-direction:column; min-width:0; }
	.vol { display:flex; align-items:center; gap:10px; padding:4px 0 12px; }
	.vol input { flex:1; }
	.icon { display:grid; place-items:center; padding:6px; }
	.pct { min-width:5ch; text-align:right; font-size:12px; color:var(--muted); }
	.mute { flex:none; }
	.secondary { background:var(--surface-2); color:var(--fg); border:1px solid var(--line); border-radius:10px; padding:6px 12px; cursor:pointer; }
</style>
