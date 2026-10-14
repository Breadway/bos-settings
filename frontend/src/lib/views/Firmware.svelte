<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface FwDevice {
		name: string;
		version: string;
		security_database: boolean;
	}

	let devices = $state<FwDevice[] | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);

	async function refresh() {
		devices = await invoke<FwDevice[]>("get_updatable_firmware");
	}

	onMount(refresh);

	function appendLine(line: string) {
		log = [...log, line];
	}

	async function checkForUpdates() {
		log = [];
		busy = true;
		await runStreamed("fwupd_refresh", {}, appendLine);
		busy = false;
		await refresh();
	}

	async function updateAll() {
		log = [];
		busy = true;
		await runStreamed("fwupd_update", {}, appendLine);
		busy = false;
		await refresh();
	}
</script>

<ViewScaffold title="Firmware">
	<Group title="Device firmware" hint="Updates for the computer itself: the BIOS, camera, dock and other parts that support it.">
		{#if devices === null}
			<p class="muted">Loading…</p>
		{:else if devices.filter((d) => !d.security_database).length === 0}
			<p class="muted">No parts of this computer can update their firmware from here.</p>
		{:else}
			{#each devices.filter((d) => !d.security_database) as dev, i (`${dev.name}:${i}`)}
				<div class="ui-item"><div class="lb"><b>{dev.name}</b><small>Version {dev.version}</small></div></div>
			{/each}
		{/if}
		{#if devices?.some((d) => d.security_database)}
			<details class="ui-disclosure">
				<summary>Secure Boot certificate lists ({devices.filter((d) => d.security_database).length})</summary>
				<p class="muted">Lists of trusted and revoked boot software. They update together with the firmware.</p>
				{#each devices.filter((d) => d.security_database) as dev, i (`${dev.name}:${i}`)}
					<div class="ui-item"><div class="lb"><b>{dev.name}</b><small>Version {dev.version}</small></div></div>
				{/each}
			</details>
		{/if}
		<div class="foot">
			<button class="btn small" disabled={busy} onclick={checkForUpdates}>{busy ? "Working…" : "Check for firmware updates"}</button>
			{#if devices?.length}<button class="btn small primary" disabled={busy} onclick={updateAll}>Install firmware updates</button>{/if}
		</div>
		{#if log.length}<details class="ui-disclosure"><summary>Details</summary><LogView lines={log} /></details>{/if}
	</Group>
</ViewScaffold>

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; }
	.foot { display:flex; gap:8px; flex-wrap:wrap; padding:6px 0 10px; }
</style>
