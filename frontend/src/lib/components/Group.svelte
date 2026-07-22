<script lang="ts">
	import type { Snippet } from "svelte";

	let {
		title,
		hint,
		wide = false,
		children,
	}: { title: string; hint?: string; wide?: boolean; children: Snippet } = $props();
</script>

<div class="group" class:wide>
	<h2 class="title">{title}</h2>
	{#if hint}<p class="hint">{hint}</p>{/if}
	<div class="rows">
		{@render children()}
	</div>
</div>

<style>
	.group {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.group.wide {
		grid-column: 1 / -1;
	}

	.title {
		font-weight: 600;
		font-size: 1.05em;
		margin: 0 0 var(--space-sm, 8px);
	}

	.hint {
		opacity: 0.75;
		font-size: var(--font-size-secondary, 12px);
		line-height: 1.4;
		margin: 0 0 var(--space-sm, 8px);
	}

	.rows {
		display: flex;
		flex-direction: column;
		/* Flex children default to refusing to shrink below their content's
		   natural width (min-width: auto) — without this, a button/input
		   with enough text overflows past the grid column's actual pixel
		   width instead of wrapping, visually spilling into the next
		   column. */
		min-width: 0;
	}

	.rows > :global(*) {
		min-width: 0;
		max-width: 100%;
		box-sizing: border-box;
	}
</style>
