<script lang="ts">
	import Row from "./Row.svelte";
	import Switch from "./Switch.svelte";

	let {
		label,
		hint,
		value = $bindable(),
        disabled = false,
	}: { label: string; hint?: string; value: boolean; disabled?: boolean } = $props();
</script>

<Row {label} {hint}>
	<Switch {disabled} bind:value ariaLabel={label} />
</Row>
