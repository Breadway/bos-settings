<script lang="ts">
	import { SIDEBAR_SECTIONS } from "$lib/sidebar";

	let { activePage = $bindable() }: { activePage: string } = $props();
</script>

<nav class="sidebar">
	{#each SIDEBAR_SECTIONS as section (section.title ?? "untitled")}
		{#if section.title}
			<div class="section-header">{section.title}</div>
		{/if}
		{#each section.items as item (item.id)}
			<button
				class="row"
				class:selected={activePage === item.id}
				onclick={() => (activePage = item.id)}
			>
				<item.icon size={16} />
				<span class="label">{item.label}</span>
			</button>
		{/each}
	{/each}
</nav>

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 220px;
		flex-shrink: 0;
		min-height: 0;
		background-color: var(--surface);
		color: var(--on-surface);
		overflow-y: auto;
		overscroll-behavior: contain;
		padding: var(--space-sm, 8px);
		gap: 1px;
	}

	.section-header {
		padding: var(--space-lg, 16px) var(--space-sm, 8px) var(--space-xs, 4px);
		font-size: 11px;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		opacity: 0.5;
	}

	.section-header:first-child {
		padding-top: var(--space-xs, 4px);
	}

	.row {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		padding: 9px var(--space-md, 12px);
		border-radius: var(--radius-secondary, 6px);
		transition: background-color 0.1s ease;
	}

	.row:hover {
		background-color: color-mix(in srgb, var(--on-surface) 8%, transparent);
	}

	.row.selected {
		background-color: var(--accent);
		color: var(--on-accent);
		font-weight: 500;
	}

	.label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
