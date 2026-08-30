<script lang="ts">
	import Row from "./Row.svelte";

	let {
		label,
		hint,
		value = $bindable(),
		min,
		max,
		step = 1,
		suffix = "",
		onChange,
	}: {
		label: string;
		hint?: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		suffix?: string;
		onChange?: () => void;
	} = $props();

	let shown = $derived(step < 1 ? Number(value).toFixed(2).replace(/0+$/, "").replace(/\.$/, "") : String(Math.round(value)));
</script>

<Row {label} {hint}>
	<input type="range" {min} {max} {step} bind:value oninput={() => onChange?.()} />
	<span class="val">{shown}{suffix}</span>
</Row>

<style>
	input[type="range"] {
		width: 160px;
		accent-color: var(--accent);
	}

	.val {
		min-width: 4.5ch;
		text-align: right;
		font-size: 12px;
		color: var(--muted);
	}
</style>
