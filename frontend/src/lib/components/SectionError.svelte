<script lang="ts">
	let { error, reset }: { error: unknown; reset: () => void } = $props();
	const message = $derived(error instanceof Error ? error.message : String(error));
</script>

<div class="section-error" role="alert">
	<b>This section couldn't load.</b>
	<span>{message}</span>
	<button class="btn" onclick={reset}>Try again</button>
</div>

<style>
	.section-error {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px 12px;
		padding: 12px 14px;
		border-radius: var(--radius, 14px);
		border: 1px solid color-mix(in oklab, var(--red, #e0736a) 45%, transparent);
		background: color-mix(in oklab, var(--red, #e0736a) 10%, transparent);
		font-size: 13px;
	}
	.section-error span {
		flex: 1 1 240px;
		color: var(--muted);
		font-family: ui-monospace, monospace;
		font-size: 12px;
		word-break: break-word;
	}
</style>
