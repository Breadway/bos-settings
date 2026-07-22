<script lang="ts">
	import Row from "./Row.svelte";

	let {
		label,
		value = $bindable(),
		min,
		max,
		step = 1,
	}: { label: string; value: number; min: number; max: number; step?: number } = $props();
</script>

<Row {label}>
	<input type="number" bind:value {min} {max} {step} />
</Row>

<style>
	input {
		width: 10ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
