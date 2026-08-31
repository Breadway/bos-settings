<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";

	interface FwDevice {
		name: string;
		version: string;
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
	<Group
		title="Updatable devices"
		hint="UEFI and devices that speak fwupd."
		wide
	>
		<div class="list">
			{#if devices === null}
				<Hint text="Loading…" />
			{:else if devices.length === 0}
				<EmptyState icon={RefreshCw} title="No updatable firmware devices found" hint="Not every device supports firmware updates through fwupd." />
			{:else}
				{#each devices as dev (dev.name)}
					<div class="row">
						<span class="name" title={dev.name}>{dev.name}</span>
						<span class="version" title={dev.version}>{dev.version}</span>
					</div>
				{/each}
			{/if}
		</div>

		<div class="btn-row">
			<button disabled={busy} onclick={checkForUpdates}>Check for updates</button>
			<button disabled={busy} class="primary" onclick={updateAll}>Update all</button>
			<button disabled={busy} onclick={refresh}>Refresh list</button>
		</div>
	</Group>

	<LogView lines={log} />
</ViewScaffold>

<style>
	.list {
		max-height: 320px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
		min-width: 0;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.version {
		opacity: 0.75;
		font-size: var(--font-size-secondary, 12px);
		flex-shrink: 0;
		max-width: 40%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.btn-row {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-top: var(--space-sm, 8px);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
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
