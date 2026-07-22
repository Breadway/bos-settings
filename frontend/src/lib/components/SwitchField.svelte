<script lang="ts">
	import Row from "./Row.svelte";
	import Switch from "./Switch.svelte";

	let { label, value = $bindable() }: { label: string; value: boolean } = $props();
</script>

<Row {label}>
	<Switch bind:value ariaLabel={label} />
</Row>
