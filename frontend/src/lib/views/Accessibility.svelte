<script lang="ts">
	import type {DesktopPreferences} from "$lib/desktopPreferences";
	import {useDraft} from "$lib/drafts.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import {refreshTheme} from "$lib/theme";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import { debounce } from "$lib/debounce";
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
	let lastZoom = $state(2);
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
		if (running && !st?.orca_installed) {
			await install(["orca"]);
			if (!st?.orca_installed) { message = "Orca couldn't be installed. Open Details to see why."; return; }
		}
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
	<Group title="Seeing">
		{#if st}
			<SwitchField label="Zoom" hint="Enlarges everything around the pointer." bind:value={() => zoom > 1, (v) => { zoom = v ? Math.max(lastZoom, 1.5) : 1; liveZoom(); }} />
			{#if zoom > 1}
				<SliderField label="Zoom level" bind:value={zoom} min={1.25} max={8} step={0.25} format={(v) => `${Math.round(v * 100)}%`} onChange={() => { lastZoom = zoom; liveZoom(); }} />
			{/if}
		{/if}
		{#if prefs}
			<Row label="Text size">
				<div class="ui-seg">{#each [[14, "Default"], [16, "Large"], [18, "Larger"]] as [v, l] (v)}<button class:on={prefs.text_size === v} onclick={() => (prefs!.text_size = Number(v))}>{l}</button>{/each}</div>
			</Row>
			<Row label="Cursor size">
				<div class="ui-seg">{#each [[24, "Default"], [32, "Large"], [48, "Larger"]] as [v, l] (v)}<button class:on={prefs.cursor_size === v} onclick={() => (prefs!.cursor_size = Number(v))}>{l}</button>{/each}</div>
			</Row>
			<SwitchField label="Stronger window borders" bind:value={prefs.stronger_borders} />
		{/if}
	</Group>
	{#if prefs}
		<Group title="Motion"><SwitchField label="Reduce motion" hint="Turns off window and workspace animations." bind:value={prefs.reduce_motion} /></Group>
	{/if}
	<Group title="Hearing and reading">
		{#if st}
			<SwitchField
				label="Read the screen aloud"
				hint={st.orca_installed ? "Uses the Orca screen reader." : "Uses the Orca screen reader. Installs it the first time, which needs your password."}
				disabled={busy}
				bind:value={() => st!.orca_running, (v) => toggleOrca(v)}
			/>
		{/if}
	</Group>
	{#if st}
		<details class="ui-disclosure">
			<summary>Other tools</summary>
			<div class="ui-item">
				<div class="lb"><b>Magnifier window</b><small>KMag shows a zoomed copy of part of the screen in its own window.</small></div>
				{#if st.kmag_installed}<button class="btn small" onclick={openKmag}>Open</button>{:else}<button class="btn small" disabled={busy} onclick={() => install(["kmag"])}>Install</button>{/if}
			</div>
		</details>
	{/if}
	{#if message}<p class="err" role="alert">{message}</p>{/if}
	{#if log.length}<details class="ui-disclosure"><summary>Details</summary><LogView lines={log} /></details>{/if}
</ViewScaffold>

<style>
	.err { color:var(--red); font-size:12.5px; margin:0 2px; }
</style>
