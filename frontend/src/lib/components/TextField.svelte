<script lang="ts">
	import Row from "./Row.svelte";

	let {
		label,
		value = $bindable(),
		placeholder = "",
	}: { label: string; value: string; placeholder?: string } = $props();
</script>

<Row {label}>
	<input type="text" bind:value {placeholder} />
</Row>

<style>
	input {
		width: 28ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-sm, 10px);
		padding: 6px 10px;
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
