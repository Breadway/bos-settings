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
		format,
	}: {
		label: string;
		hint?: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		suffix?: string;
		onChange?: () => void;
		/** Turns the raw value into words, e.g. 1 -> "100%" or 0 -> "Normal". */
		format?: (v: number) => string;
	} = $props();

	let shown = $derived(format ? format(Number(value)) : step < 1 ? Number(value).toFixed(2).replace(/0+$/, "").replace(/\.$/, "") : String(Math.round(value)));
</script>

<Row {label} {hint}>
	<input aria-label={label} type="range" {min} {max} {step} bind:value oninput={() => onChange?.()} />
	<span class="val">{shown}{format ? "" : suffix}</span>
</Row>

<style>
	input[type="range"] {
		width: clamp(80px, 22vw, 180px);
		accent-color: var(--accent);
	}

	.val {
		min-width: 4.5ch;
		text-align: right;
		font-size: 12px;
		color: var(--muted);
	}
</style>
