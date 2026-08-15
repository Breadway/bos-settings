<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import LogView from "$lib/components/LogView.svelte";

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

	async function apply(enabled: boolean) {
		if (!st) return;
		message = "";
		try {
			st = await invoke<NightlightStatus>("set_nightlight", {
				enabled,
				temperature: st.temperature,
			});
			if (st.error) message = st.error;
		} catch (e) {
			message = `${e}`;
		}
	}

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
	<Group
		title="Hyprland twilight"
		hint="hyprsunset owns the compositor color filter. hyprctl hyprsunset talks to its socket — the binary has to be installed and running."
	>
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.installed}
			<Hint text="hyprsunset is not installed. Night light cannot run without it." />
			<button class="primary" disabled={busy} onclick={install}>Install hyprsunset</button>
		{:else}
			<div class="row-switch">
				<span>Night light</span>
				<button
					class="switch"
					class:on={st.enabled}
					role="switch"
					aria-checked={st.enabled}
					aria-label="Night light"
					onclick={() => apply(!st!.enabled)}
				>
					<span class="knob"></span>
				</button>
			</div>
			<NumberField label="Temperature (K)" bind:value={st.temperature} min={2000} max={6500} step={100} />
			<button disabled={!st.enabled} onclick={() => apply(true)}>Apply temperature</button>
			<Hint text={st.running ? "hyprsunset is running." : "hyprsunset will start when you enable night light."} />
		{/if}
		{#if message}<Hint text={message} />{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>

<style>
	.row-switch {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.switch {
		width: 40px;
		height: 22px;
		flex-shrink: 0;
		border-radius: 999px;
		border: none;
		background-color: var(--overlay);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
	}

	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}

	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
