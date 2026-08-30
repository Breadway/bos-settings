<script lang="ts">
	import Search from "@lucide/svelte/icons/search";
	import { SIDEBAR_SECTIONS } from "$lib/sidebar";
	import { searchSettings } from "$lib/search";
	import { go, nav } from "$lib/nav.svelte";

	let query = $state("");
	let hits = $derived(query.trim().length >= 1 ? searchSettings(query, 6) : []);

	function jump(page: string, tab?: string) {
		query = "";
		go(page, tab);
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === "Enter" && hits[0]) {
			jump(hits[0].page, hits[0].tab);
		}
	}
</script>

<nav class="sidebar">
	<div class="search-wrap">
		<Search size={16} />
		<input bind:value={query} placeholder="Search settings" onkeydown={onKey} />
	</div>

	{#if hits.length > 0}
		<div class="hits">
			{#each hits as hit (`${hit.page}:${hit.tab ?? ""}:${hit.label}`)}
				<button type="button" class="hit" onclick={() => jump(hit.page, hit.tab)}>
					<span>{hit.label}</span>
				</button>
			{/each}
		</div>
	{:else}
		<div class="nav">
			{#each SIDEBAR_SECTIONS as section (section.title ?? "untitled")}
				{#if section.title}
					<div class="section-header">{section.title}</div>
				{/if}
				{#each section.items as item (item.id)}
					<button
						type="button"
						class="row"
						class:selected={nav.page === item.id}
						onclick={() => go(item.id)}
					>
						<item.icon size={16} />
						<span class="label">{item.label}</span>
					</button>
				{/each}
			{/each}
		</div>
	{/if}
</nav>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 248px;
		flex-shrink: 0;
		min-height: 0;
		background: var(--bg-2, var(--surface));
		color: var(--on-surface);
		border-right: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
		overflow: hidden;
		padding: 12px 10px 14px;
	}

	.search-wrap {
		position: relative;
		margin: 2px 6px 12px;
		flex-shrink: 0;
	}

	.search-wrap :global(svg) {
		position: absolute;
		left: 11px;
		top: 10px;
		opacity: 0.45;
		pointer-events: none;
	}

	.search-wrap input {
		width: 100%;
		background: var(--surface);
		border: 1px solid transparent;
		border-radius: 10px;
		padding: 9px 10px 9px 34px;
		color: var(--fg);
	}

	.search-wrap input:focus {
		border-color: color-mix(in oklab, var(--accent) 55%, transparent);
	}

	.nav,
	.hits {
		overflow-y: auto;
		overscroll-behavior: contain;
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 0 4px;
		min-height: 0;
	}

	.section-header {
		padding: 14px 10px 6px;
		font-size: 10px;
		font-weight: 700;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--faint, color-mix(in oklab, var(--fg) 38%, transparent));
	}

	.section-header:first-child {
		padding-top: 4px;
	}

	.row,
	.hit {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		padding: 8px 10px;
		border-radius: 10px;
	}

	.row:hover,
	.hit:hover {
		background: color-mix(in srgb, var(--fg) 6%, transparent);
	}

	.row.selected {
		background: var(--accent-soft, color-mix(in oklab, var(--accent) 18%, transparent));
		color: var(--fg);
		font-weight: 500;
	}

	.row.selected :global(svg) {
		color: var(--accent);
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 13.5px;
	}
</style>
