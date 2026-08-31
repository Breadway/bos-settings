<script lang="ts">
	let {
		items,
		value = $bindable(),
	}: { items: { id: string; label: string }[]; value: string } = $props();
</script>

<div class="subnav">
	{#each items as item (item.id)}
		<button type="button" class:on={value === item.id} onclick={() => (value = item.id)}>
			{item.label}
		</button>
	{/each}
</div>

<style>
	.subnav {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: -8px 0 4px;
	}

	button {
		padding: 6px 11px;
		border-radius: 999px;
		font-size: 12px;
		background: color-mix(in srgb, var(--fg) 6%, transparent);
		border: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
		color: var(--fg);
		cursor: pointer;
	}

	button.on {
		background: var(--accent);
		color: var(--on-accent);
		border-color: transparent;
		font-weight: 600;
	}
</style>
