<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import { debounce } from "$lib/debounce";

	interface NightlightStatus {
		installed: boolean;
		running: boolean;
		enabled: boolean;
		temperature: number;
		error: string | null;
	}

	let st = $state<NightlightStatus | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");
	async function refresh() {
		st = await invoke<NightlightStatus>("get_nightlight");
	}

	onMount(refresh);

	async function applyNow() {
		if (!st) return;
		message = "";
		try {
			st = await invoke<NightlightStatus>("set_nightlight", {
				enabled: st.enabled,
				temperature: st.temperature,
			});
			if (st.error) message = st.error;
		} catch (e) {
			message = `${e}`;
		}
	}

	const persistTemp = debounce(applyNow, 200);

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
	<Group title="Night light" hint="Warmer screen after dark.">
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.installed}
			<Hint text="hyprsunset is not installed." />
			<button class="btn primary" disabled={busy} onclick={install}>Install</button>
		{:else}
			<SwitchField
				label="Night light"
				bind:value={() => st!.enabled, (v) => { st!.enabled = v; applyNow(); }}
			/>
			{#if st.enabled}
				<SliderField
					label="Warmth"
					bind:value={st.temperature}
					min={2000}
					max={6500}
					step={100}
					suffix="K"
					onChange={persistTemp}
				/>
			{/if}
		{/if}
		{#if message}<Hint text={message} />{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>
