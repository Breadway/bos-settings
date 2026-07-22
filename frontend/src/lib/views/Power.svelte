<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Row from "$lib/components/Row.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";

	interface PowerInfo {
		battery: [string, string][];
		power_source: string;
		brightness_pct: number | null;
		charge_start: number | null;
		charge_end: number | null;
		tlp_profile: string | null;
	}

	let info = $state<PowerInfo | null>(null);
	let brightness = $state(0);
	let chargeStart = $state(0);
	let chargeEnd = $state(100);

	onMount(async () => {
		info = await invoke<PowerInfo>("get_power_info");
		brightness = info.brightness_pct ?? 0;
		chargeStart = info.charge_start ?? 0;
		chargeEnd = info.charge_end ?? 100;
	});

	async function setBrightness(percent: number) {
		brightness = percent;
		await invoke("set_brightness", { percent });
	}

	async function setChargeThreshold(which: "start" | "end", percent: number) {
		if (which === "start") chargeStart = percent;
		else chargeEnd = percent;
		await invoke("set_charge_threshold", { which, percent });
	}
</script>

<ViewScaffold title="Power">
	{#if info}
		<Group title="Battery">
			{#each info.battery as [label, value] (label)}
				<InfoRow {label} {value} />
			{/each}
			<InfoRow label="Power source" value={info.power_source} />
		</Group>

		<Group title="Brightness">
			{#if info.brightness_pct !== null}
				<Row label="Screen brightness">
					<input type="range" min="1" max="100" bind:value={brightness} onchange={() => setBrightness(brightness)} />
					<span class="pct">{brightness}%</span>
				</Row>
			{:else}
				<Hint text="No controllable backlight found." />
			{/if}
		</Group>

		{#if info.charge_start !== null && info.charge_end !== null}
			<Group
				title="Charge limits"
				hint="Some laptops let you cap charging below 100% to slow battery wear on a machine that's mostly plugged in."
			>
				<Row label="Start charging below (%)">
					<input type="number" min="0" max="100" bind:value={chargeStart} onchange={() => setChargeThreshold("start", chargeStart)} />
				</Row>
				<Row label="Stop charging at (%)">
					<input type="number" min="1" max="100" bind:value={chargeEnd} onchange={() => setChargeThreshold("end", chargeEnd)} />
				</Row>
			</Group>
		{/if}

		<Group
			title="TLP"
			hint="TLP automatically applies a power-saving profile on battery and a performance profile on AC — there's no manual switch by design."
		>
			<InfoRow label="Current profile" value={info.tlp_profile ?? "unknown"} />
		</Group>
	{/if}
</ViewScaffold>

<style>
	input[type="range"] {
		width: 180px;
	}

	input[type="number"] {
		width: 8ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.pct {
		margin-left: var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.7;
	}
</style>
