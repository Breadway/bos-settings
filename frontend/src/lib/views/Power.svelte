<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Row from "$lib/components/Row.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface BatteryProtection {
		kind: "thresholds" | "charge_types" | "conservation";
		enabled: boolean;
		limit_pct: number | null;
	}

	interface PowerInfo {
		protection: BatteryProtection | null;
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

	const PROTECT_STOP = 80;
	const PROTECT_RESUME = 75;
	let protectMsg = $state("");

	// One switch over whichever mechanism the hardware has (thresholds,
	// charge_types Long_Life, or ideapad conservation_mode). The switch only
	// moves once the privileged write succeeded; a refused or cancelled
	// authentication leaves it where it was and says why.
	async function setProtection(on: boolean) {
		if (!info?.protection) return;
		protectMsg = "";
		const before = info.protection.enabled;
		info.protection.enabled = on;
		try {
			info.protection = await invoke<BatteryProtection>("set_battery_protection", { enabled: on });
			if (info.protection.limit_pct !== null) chargeEnd = info.protection.limit_pct;
		} catch (e) {
			info.protection.enabled = before;
			protectMsg = `${e}`;
		}
	}

	async function setChargeThreshold(which: "start" | "end", percent: number) {
		if (which === "start") chargeStart = percent;
		else chargeEnd = percent;
		await invoke("set_charge_threshold", { which, percent });
	}
</script>

<ViewScaffold
	title="Power & battery"
	lede="Brightness, battery, and charge limits."
>
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

		{#if info.protection}
			<Group
				title="Battery protection"
				hint={info.protection.kind === "thresholds"
					? `Stops charging at ${PROTECT_STOP}% and resumes at ${PROTECT_RESUME}%. Needs your password.`
					: "Stops charging at about 80% to make the battery last longer. Needs your password."}
			>
				<SwitchField
					label="Protect battery"
					hint={info.protection.enabled ? "On: charging is limited." : "Off: charges to 100%."}
					bind:value={() => info!.protection!.enabled, (v) => setProtection(v)}
				/>
				{#if protectMsg}<Hint text={protectMsg} />{/if}
			</Group>
		{/if}

		{#if info.charge_start !== null && info.charge_end !== null}
			<Group
				title="Charge limits"
				hint="Keeps the battery off 100% when this machine stays plugged in."
			>
				<Row label="Start charging below">
					<input type="range" min="0" max="100" bind:value={chargeStart} onchange={() => setChargeThreshold("start", chargeStart)} />
					<span class="pct">{chargeStart}%</span>
				</Row>
				<Row label="Stop charging at">
					<input type="range" min="1" max="100" bind:value={chargeEnd} onchange={() => setChargeThreshold("end", chargeEnd)} />
					<span class="pct">{chargeEnd}%</span>
				</Row>
			</Group>
		{/if}

		<Group
			title="TLP"
			hint="TLP picks battery vs AC on its own."
		>
			<InfoRow label="Current profile" value={info.tlp_profile ?? "unknown"} />
		</Group>
	{/if}
</ViewScaffold>

<style>
	input[type="range"] {
		width: 180px;
	}

	.pct {
		margin-left: var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.7;
	}
</style>
