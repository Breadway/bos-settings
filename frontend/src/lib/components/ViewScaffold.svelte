<script lang="ts">
	import type { Snippet } from "svelte";

	let { title, children }: { title: string; children: Snippet } = $props();
</script>

<div class="view">
	<h1 class="title">{title}</h1>
	<div class="content">
		{@render children()}
	</div>
</div>

<style>
	.view {
		padding: var(--space-xl, 20px) var(--space-xl, 20px) 48px;
		height: 100%;
		overflow-y: auto;
	}

	.title {
		font-size: 1.6em;
		font-weight: bold;
		margin: 0 0 var(--space-xl, 20px);
	}

	/* Groups lay out as a responsive grid rather than a single narrow
	   column — on a wide (tiled/maximized) window that means multiple
	   independent setting groups sit side by side instead of one column
	   with a wall of empty space either side. `.group.wide` (device
	   lists, thumbnail grids, log views — anything that reads badly
	   squeezed into a card) opts out via `grid-column: 1 / -1`. */
	.content {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
		gap: var(--space-xl, 20px) 32px;
		align-items: start;
		max-width: 1400px;
	}
</style>
