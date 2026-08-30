<script lang="ts">
	import type { Snippet } from "svelte";

	let { label, hint, children }: { label: string; hint?: string; children: Snippet } = $props();
</script>

<div class="field-row">
	<div class="meta">
		<span class="label">{label}</span>
		{#if hint}<span class="hint">{hint}</span>{/if}
	</div>
	<div class="control">
		{@render children()}
	</div>
</div>

<style>
	.field-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 11px 2px;
		min-height: 24px;
	}

	:global(.field-row + .field-row) {
		border-top: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
	}

	.meta {
		flex: 1;
		min-width: 0;
	}

	.label {
		display: block;
	}

	.hint {
		display: block;
		font-size: 12px;
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		margin-top: 2px;
	}

	.control {
		display: flex;
		align-items: center;
		flex-shrink: 0;
	}
</style>
