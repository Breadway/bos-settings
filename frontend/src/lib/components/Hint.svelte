<script lang="ts">
	let { text }: { text: string } = $props();
</script>

<p class="hint">{text}</p>

<style>
	.hint {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
		margin: 0 0 var(--space-xs, 4px);
		/* No-op unless this Hint is a direct child of ViewScaffold's CSS
		   grid (i.e. used standalone between Groups, not nested inside
		   one) — in that case it spans full width instead of getting
		   squeezed into whichever grid column it happened to land in. */
		grid-column: 1 / -1;
	}
</style>
