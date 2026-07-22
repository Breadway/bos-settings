<script lang="ts">
	let { lines }: { lines: string[] } = $props();
</script>

{#if lines.length > 0}
	<pre class="log">{lines.join("\n")}</pre>
{/if}

<style>
	.log {
		background-color: var(--surface);
		color: var(--on-surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px);
		font-family: monospace;
		font-size: 12px;
		max-height: 200px;
		overflow-y: auto;
		white-space: pre-wrap;
		word-break: break-all;
		margin-top: var(--space-sm, 8px);
		grid-column: 1 / -1;
	}
</style>
