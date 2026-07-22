<script lang="ts">
	import Row from "./Row.svelte";

	let { label, value = $bindable(), placeholder = "" }: { label: string; value: string[]; placeholder?: string } =
		$props();

	let text = $state(value.join(", "));

	function onInput() {
		value = text
			.split(",")
			.map((s) => s.trim())
			.filter((s) => s.length > 0);
	}
</script>

<Row {label}>
	<input type="text" bind:value={text} oninput={onInput} {placeholder} />
</Row>

<style>
	input {
		width: 28ch;
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
