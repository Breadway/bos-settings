<script lang="ts">
	import Row from "./Row.svelte";

	let {
		label,
		hint,
		value = $bindable(),
		options,
		labels = {},
	}: {
		label: string;
		hint?: string;
		value: string;
		options: string[];
		labels?: Record<string, string>;
	} = $props();
</script>

<Row {label} {hint}>
	<select bind:value>
		{#each options as opt (opt)}
			<option value={opt}>{labels[opt] ?? opt}</option>
		{/each}
	</select>
</Row>

<style>
	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-sm, 10px);
		padding: 6px 10px;
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
