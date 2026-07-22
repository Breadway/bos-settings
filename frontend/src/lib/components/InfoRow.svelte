<script lang="ts">
	import Row from "./Row.svelte";

	let { label, value }: { label: string; value: string } = $props();
</script>

<Row {label}>
	<span class="value">{value}</span>
</Row>

<style>
	.value {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
