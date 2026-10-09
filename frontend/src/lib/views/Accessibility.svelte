<script lang="ts">
	import type {DesktopPreferences} from "$lib/desktopPreferences";
	import {useDraft} from "$lib/drafts.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import {refreshTheme} from "$lib/theme";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import { debounce } from "$lib/debounce";
	import NumberField from "$lib/components/NumberField.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface A11yStatus {
		orca_installed: boolean;
		orca_running: boolean;
		zoom_factor: number;
		sticky_keys_supported: boolean;
		slow_keys_supported: boolean;
		kmag_installed: boolean;
		note: string;
	}

	let st = $state<A11yStatus | null>(null);
	let zoom = $state(1);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");

	async function refresh() {
		st = await invoke<A11yStatus>("get_a11y_status");
		zoom = st.zoom_factor;
	}

	let prefs=$state<DesktopPreferences|null>(null);
	onMount(async()=>{await refresh();prefs=await invoke("get_desktop_preferences")});
	async function savePreferences(){await invoke("save_desktop_preferences",{preferences:prefs});await refreshTheme();}
	useDraft(()=>prefs,savePreferences,()=>prefs!==null);

	async function install(packages: string[]) {
		log = [];
		busy = true;
		await runStreamed("pacman_install", { packages }, (line) => {
			log = [...log, line];
		});
		busy = false;
		await refresh();
	}

	async function toggleOrca(running: boolean) {
		message = "";
		try {
			await invoke("set_orca_running", { running });
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
	}


 let zoomTask = Promise.resolve();
 const liveZoom = debounce(() => { void applyZoom(); }, 180);
 function applyZoom() {
  const requested = zoom;
  zoomTask = zoomTask.then(async () => {
   message = "";
   try {
    const applied = await invoke<number>("set_cursor_zoom", {factor:requested});
    if(st) st.zoom_factor = applied;
    if(zoom === requested) zoom = applied;
   } catch(e) {
    message = String(e);
    if(zoom === requested && st) zoom = st.zoom_factor;
   }
  });
  return zoomTask;
 }

	async function openKmag() {
		message = "";
		try {
			await invoke("open_kmag");
		} catch (e) {
			message = `${e}`;
		}
	}
</script>

<ViewScaffold title="Accessibility" lede="Make BOS easier to see, hear and use.">
 {#if prefs}
 <Group title="Seeing">
  <SelectField label="Text size" bind:value={() => String(prefs!.text_size), v => prefs!.text_size=Number(v)} options={['12','14','16','18']} labels={{'12':'Small','14':'Default','16':'Large','18':'Larger'}}/>
  <SelectField label="Cursor size" bind:value={() => String(prefs!.cursor_size), v => prefs!.cursor_size=Number(v)} options={['24','32','48']} labels={{'24':'Default','32':'Large','48':'Larger'}}/>
  <SwitchField label="Stronger window borders" bind:value={prefs.stronger_borders}/>
 </Group>
 <Group title="Motion"><SwitchField label="Reduce motion" hint="Turns off window and workspace animations." bind:value={prefs.reduce_motion}/></Group>
 {/if}
	<Group title="Screen reader" hint="Orca. Works on Wayland; start/stop is process-level, not a compositor setting.">
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.orca_installed}
			<Hint text="orca is not installed." />
			<button class="primary" disabled={busy} onclick={() => install(["orca"])}>Install orca</button>
		{:else}
			<div class="row-switch">
				<span>Orca</span>
				<button
					class="switch"
					class:on={st.orca_running}
					role="switch"
					aria-checked={st.orca_running}
					aria-label="Orca"
					onclick={() => toggleOrca(!st!.orca_running)}
				>
					<span class="knob"></span>
				</button>
			</div>
		{/if}
	</Group>

	<Group title="Magnifier" hint="Zooms around the cursor. This session only.">
		{#if st}
			<SliderField label="Zoom level" bind:value={zoom} min={1} max={8} step={0.25} suffix="×" onChange={liveZoom} />
			<NumberField label="Zoom factor" bind:value={zoom} min={1} max={8} step={0.25} />
			<button class="primary" onclick={applyZoom}>Apply zoom</button>
			<Hint text="1.0 is off. Values above 1 enlarge around the cursor." />
			{#if !st.kmag_installed}
				<Hint text="kmag is a separate KDE magnifier and is not wired into Hyprland. Optional install only." />
				<button disabled={busy} onclick={() => install(["kmag"])}>Install kmag</button>
			{:else}
				<button onclick={openKmag}>Open kmag</button>
			{/if}
		{/if}
	</Group>

	<Group title="Sticky keys / Slow keys">
		{#if st}
			<div class="row-switch disabled">
				<span>Sticky keys</span>
				<button class="switch" disabled role="switch" aria-checked="false" aria-label="Sticky keys">
					<span class="knob"></span>
				</button>
			</div>
			<div class="row-switch disabled">
				<span>Slow keys</span>
				<button class="switch" disabled role="switch" aria-checked="false" aria-label="Slow keys">
					<span class="knob"></span>
				</button>
			</div>
			<Hint text={st.note} />
		{/if}
	</Group>
	{#if message}<Hint text={message} />{/if}
	<LogView lines={log} />
</ViewScaffold>

<style>
	.row-switch {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.row-switch.disabled {
		opacity: 0.55;
	}

	.switch {
		width: 40px;
		height: 22px;
		flex-shrink: 0;
		border-radius: 999px;
		border: none;
		background-color: var(--overlay);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
	}

	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}

	.switch:disabled {
		cursor: not-allowed;
	}

	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		align-self: flex-start;
		margin-top: var(--space-xs, 4px);
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
