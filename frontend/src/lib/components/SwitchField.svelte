<script lang="ts">
	import Row from "./Row.svelte";
	import Switch from "./Switch.svelte";

	let {
		label,
		hint,
		value = $bindable(),
	}: { label: string; hint?: string; value: boolean } = $props();
</script>

<Row {label} {hint}>
	<Switch bind:value ariaLabel={label} />
</Row>
