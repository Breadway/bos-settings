<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import Search from "@lucide/svelte/icons/search";

	interface AurResult {
		name: string;
		version: string;
		description: string;
	}

	let query = $state("");
	let results = $state<AurResult[] | null>(null);
	let status = $state("Search for a package to see results here.");
	let searching = $state(false);

	async function search() {
		if (!query.trim()) return;
		searching = true;
		status = "Searching…";
		results = await invoke<AurResult[]>("search_aur", { query });
		status = results.length === 0 ? "No results." : `${results.length} result(s)`;
		searching = false;
	}

	function install(pkg: string) {
		invoke("install_aur_package", { pkg });
	}
</script>

<ViewScaffold title="AUR">
	<Group
		title="Search"
		hint="Search the Arch User Repository via yay. Installing opens a terminal — AUR packages run arbitrary build scripts, and reviewing what yay is about to do (and entering your password) is a real safety step."
		wide
	>
		<div class="search-row">
			<input type="text" bind:value={query} placeholder="Search the AUR…" onkeydown={(e) => e.key === "Enter" && search()} />
			<button disabled={searching} onclick={search}>Search</button>
		</div>

		<Hint text={status} />

		<div class="results">
			{#if results && results.length === 0}
				<EmptyState icon={Search} title="No results" hint="Try a different search term." />
			{:else if results}
				{#each results as r (r.name)}
					<div class="card">
						<div class="top">
							<span class="name" title={r.name}>{r.name}</span>
							<span class="version" title={r.version}>{r.version}</span>
							<button class="install" onclick={() => install(r.name)}>Install</button>
						</div>
						<span class="desc">{r.description}</span>
					</div>
				{/each}
			{/if}
		</div>
	</Group>
</ViewScaffold>

<style>
	.search-row {
		display: flex;
		gap: var(--space-sm, 8px);
	}

	.search-row input {
		flex: 1;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.search-row button {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-lg, 16px);
		cursor: pointer;
	}

	.search-row button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.results {
		max-height: 400px;
		overflow-y: auto;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: 6px;
		align-content: start;
	}

	.card {
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}

	.top {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
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

	.desc {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.install {
		background-color: var(--bg);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}
</style>
