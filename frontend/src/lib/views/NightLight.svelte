<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import Row from "$lib/components/Row.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import { debounce } from "$lib/debounce";
	import { kelvinToWarmth, warmthToKelvin, WARM_GRADIENT } from "$lib/nightlight";

	type Mode = "sunset" | "custom" | "always";

	interface NightlightStatus {
		installed: boolean;
		running: boolean;
		enabled: boolean;
		temperature: number;
		mode: Mode;
		start: string;
		end: string;
		sunrise: string | null;
		sunset: string | null;
		location: string | null;
		error: string | null;
	}

	const MODES: { id: Mode; label: string }[] = [
		{ id: "sunset", label: "Sunset to sunrise" },
		{ id: "custom", label: "Custom hours" },
		{ id: "always", label: "Always on" },
	];

	let st = $state<NightlightStatus | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");
	// The slider position is warmth (right = warmer = lower Kelvin); the
	// Kelvin value shown and sent to the backend is derived from it.
	let warmth = $state(0);
	let kelvin = $derived(warmthToKelvin(warmth));

	async function refresh() {
		st = await invoke<NightlightStatus>("get_nightlight");
		warmth = kelvinToWarmth(st.temperature);
		if (st.error) message = st.error;
	}

	onMount(refresh);

	async function apply(patch: Partial<Pick<NightlightStatus, "enabled" | "mode" | "start" | "end">> = {}) {
		if (!st) return;
		const before = $state.snapshot(st);
		const next = { ...before, ...patch };
		message = "";
		try {
			st = await invoke<NightlightStatus>("set_nightlight", {
				enabled: next.enabled,
				temperature: warmthToKelvin(warmth),
				mode: next.mode,
				start: next.start,
				end: next.end,
			});
			warmth = kelvinToWarmth(st.temperature);
			if (st.error) message = st.error;
		} catch (e) {
			// Roll the optimistic state back and say why.
			st = before;
			message = `${e}`;
		}
	}

	const persistTemp = debounce(() => apply(), 250);

	async function install() {
		log = [];
		busy = true;
		await runStreamed("pacman_install", { packages: ["hyprsunset"] }, (line) => {
			log = [...log, line];
		});
		busy = false;
		await refresh();
	}
</script>

<ViewScaffold title="Night light">
	<Group title="Night light" hint="A warmer screen after dark helps you wind down.">
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.installed}
			<Hint text="hyprsunset is not installed." />
			<button class="btn primary" disabled={busy} onclick={install}>Install</button>
		{:else}
			<SwitchField
				label="Night light"
				bind:value={() => st!.enabled, (v) => apply({ enabled: v })}
			/>
			{#if st.enabled}
				<Row label="When">
					<div class="pills" role="radiogroup" aria-label="When night light is on">
						{#each MODES as m (m.id)}
							<button
								type="button"
								class="pill"
								class:on={st.mode === m.id}
								role="radio"
								aria-checked={st.mode === m.id}
								onclick={() => apply({ mode: m.id })}
							>
								{m.label}
							</button>
						{/each}
					</div>
				</Row>
				{#if st.mode === "sunset"}
					<Hint
						text={st.sunset && st.sunrise
							? `Today in ${st.location}: on at ${st.sunset}, off at ${st.sunrise}. Times come from your time zone and update when you open this page.`
							: "Can't work out sunset for this time zone. Use custom hours."}
					/>
				{:else if st.mode === "custom"}
					<Row label="From">
						<input
							type="time"
							value={st.start}
							onchange={(e) => e.currentTarget.value && apply({ start: e.currentTarget.value })}
						/>
					</Row>
					<Row label="To">
						<input
							type="time"
							value={st.end}
							onchange={(e) => e.currentTarget.value && apply({ end: e.currentTarget.value })}
						/>
					</Row>
				{/if}
				<Row label="Warmth" hint="Less warm on the left, more warm on the right.">
					<span class="end">Less</span>
					<input
						class="warm"
						type="range"
						min="0"
						max="100"
						step="1"
						aria-label="Warmth"
						style="background: {WARM_GRADIENT};"
						bind:value={warmth}
						oninput={persistTemp}
					/>
					<span class="end">More</span>
					<span class="k">{kelvin}K</span>
				</Row>
			{/if}
		{/if}
		{#if message}<Hint text={message} />{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>

<style>
	input[type="range"].warm {
		-webkit-appearance: none;
		appearance: none;
		width: 200px;
		max-width: 100%;
		height: 6px;
		border-radius: 3px;
		cursor: pointer;
	}

	input[type="range"].warm::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: #fff;
		box-shadow: 0 1px 4px #0008;
	}

	.end {
		font-size: 12px;
		color: var(--muted);
		margin: 0 8px;
	}

	.k {
		min-width: 5ch;
		text-align: right;
		font-size: 12px;
		color: var(--muted);
	}

	input[type="time"] {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-sm, 10px);
		padding: 6px 10px;
		color-scheme: dark;
	}

	input[type="time"]:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
