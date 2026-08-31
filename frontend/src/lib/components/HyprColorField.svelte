<script lang="ts">
	import Row from "./Row.svelte";

	// Hyprland's own color format: "rgba(RRGGBBAA)"  -  a plain hex color input
	// has no alpha channel, so this pairs one with an alpha slider and
	// recombines them into that exact string on every change.
	let { label, value = $bindable() }: { label: string; value: string } = $props();

	function parse(v: string): { hex: string; alpha: number } {
		const inner = v.match(/^rgba\(([0-9a-fA-F]{8})\)$/)?.[1];
		if (!inner) return { hex: "#808080", alpha: 255 };
		return { hex: `#${inner.slice(0, 6)}`, alpha: parseInt(inner.slice(6, 8), 16) };
	}

	let parsed = $derived(parse(value));

	function update(hex: string, alpha: number) {
		const a = alpha.toString(16).padStart(2, "0");
		value = `rgba(${hex.slice(1)}${a})`;
	}
</script>

<Row {label}>
	<div class="wrap">
		<input type="color" value={parsed.hex} oninput={(e) => update(e.currentTarget.value, parsed.alpha)} />
		<input
			type="range"
			min="0"
			max="255"
			value={parsed.alpha}
			oninput={(e) => update(parsed.hex, Number(e.currentTarget.value))}
		/>
	</div>
</Row>

<style>
	.wrap {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
	}

	input[type="color"] {
		width: 36px;
		height: 26px;
		border: 1px solid color-mix(in srgb, var(--on-surface) 20%, transparent);
		border-radius: var(--radius-tertiary, 4px);
		background: transparent;
		padding: 0;
		cursor: pointer;
	}

	input[type="range"] {
		width: 100px;
	}
</style>
