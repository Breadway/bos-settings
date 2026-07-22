<script lang="ts">
	import Row from "./Row.svelte";

	let { label, value = $bindable(), options }: { label: string; value: string; options: string[] } = $props();
</script>

<Row {label}>
	<select bind:value>
		{#each options as opt (opt)}
			<option value={opt}>{opt}</option>
		{/each}
	</select>
</Row>

<style>
	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	option {
		background-color: var(--bg);
		color: var(--on-surface);
	}

	select:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
