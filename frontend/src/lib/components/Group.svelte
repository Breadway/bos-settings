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
		background: var(--surface);
		border: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
		border-radius: var(--radius, 14px);
		padding: 16px 18px;
	}

	.group.wide {
		width: 100%;
	}

	.title {
		font-weight: 600;
		font-size: 13px;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		margin: 0 0 12px;
	}

	.hint {
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		font-size: var(--font-size-secondary, 12px);
		line-height: 1.4;
		margin: -6px 0 12px;
	}

	.rows {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.rows > :global(*) {
		min-width: 0;
		max-width: 100%;
		box-sizing: border-box;
	}
</style>
