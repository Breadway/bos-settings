<script lang="ts">
	import type { Snippet } from "svelte";

	let { label, children }: { label: string; children: Snippet } = $props();
</script>

<div class="field-row">
	<span class="label">{label}</span>
	<div class="control">
		{@render children()}
	</div>
</div>

<style>
	/* GNOME Settings' "boxed list" look: consecutive rows merge into one
	   card with a thin divider between them, rounding only the outer
	   corners of the run — not each row individually. `.field-row` is a
	   distinctive name (not just `.row`) so the :global() adjacency rules
	   below can't collide with an unrelated `.row` class elsewhere (e.g.
	   Sidebar's own nav rows) — every Row instance (and everything that
	   wraps it — InfoRow, the field components) renders this same element,
	   so a run of them is just plain DOM adjacency; no parent wrapper
	   component needed. */
	.field-row {
		display: flex;
		align-items: center;
		gap: var(--space-lg, 16px);
		background-color: var(--surface);
		color: var(--on-surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
		min-height: 24px;
	}

	:global(.field-row + .field-row) {
		margin-top: calc(var(--space-sm, 8px) * -1);
		border-top: 1px solid var(--bg);
		border-top-left-radius: 0;
		border-top-right-radius: 0;
	}

	:global(.field-row:has(+ .field-row)) {
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
		margin-bottom: 0;
	}

	.label {
		flex: 1;
	}

	.control {
		display: flex;
		align-items: center;
	}
</style>
