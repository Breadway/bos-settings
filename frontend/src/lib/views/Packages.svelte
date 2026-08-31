<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import Package from "@lucide/svelte/icons/package";

	interface InstalledPackage {
		name: string;
		version: string;
	}

	let packages = $state<InstalledPackage[] | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);

	async function refresh() {
		packages = await invoke<InstalledPackage[]>("get_installed_packages");
	}

	onMount(refresh);

	function appendLine(line: string) {
		log = [...log, line];
	}

	async function updatePackage(name: string) {
		log = [];
		busy = true;
		await runStreamed("bakery_update", { name }, appendLine);
		busy = false;
		await refresh();
	}

	async function listInstalled() {
		log = [];
		busy = true;
		await runStreamed("bakery_list", {}, appendLine);
		busy = false;
	}

	async function updateAll() {
		log = [];
		busy = true;
		await runStreamed("bakery_update_all", {}, appendLine);
		busy = false;
		await refresh();
	}

	async function updateSystem() {
		log = [];
		busy = true;
		await runStreamed("pacman_system_update", {}, appendLine);
		busy = false;
	}
</script>

<ViewScaffold title="Packages">
	<Group title="Bread ecosystem (bakery)" wide>
		<div class="list">
			{#if packages === null}
				<Hint text="Loading…" />
			{:else if packages.length === 0}
				<EmptyState icon={Package} title="No bakery packages found" hint="~/.local/state/bakery/installed.json is missing or empty." />
			{:else}
				{#each packages as pkg (pkg.name)}
					<div class="row">
						<span class="name" title={pkg.name}>{pkg.name}</span>
						<span class="version" title={pkg.version}>{pkg.version}</span>
						<button disabled={busy} onclick={() => updatePackage(pkg.name)}>Update</button>
					</div>
				{/each}
			{/if}
		</div>

		<div class="btn-row">
			<button disabled={busy} onclick={listInstalled}>List installed</button>
			<button disabled={busy} onclick={updateAll}>Update all</button>
		</div>
	</Group>

	<Group
		title="System packages (pacman)"
		hint="Official repos. Needs your password."
	>
		<button disabled={busy} onclick={updateSystem}>Update system (pacman -Syu)</button>
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

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
