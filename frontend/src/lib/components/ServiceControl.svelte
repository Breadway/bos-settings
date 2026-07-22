<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Row from "./Row.svelte";
	import Group from "./Group.svelte";
	import Hint from "./Hint.svelte";

	let { unit, critical = false, hasConfig = false }: { unit: string; critical?: boolean; hasConfig?: boolean } =
		$props();

	let active = $state(false);
	let enabled = $state(false);
	let busy = $state(false);

	async function refresh() {
		const status = await invoke<{ active: boolean; enabled: boolean }>("get_service_status", { unit });
		active = status.active;
		enabled = status.enabled;
	}

	async function run(action: "Start" | "Stop" | "Restart") {
		busy = true;
		try {
			await invoke("service_action", { unit, action });
		} finally {
			busy = false;
			await refresh();
		}
	}

	function toggle() {
		if (active && critical) {
			if (!confirm(`Stop ${unit}? This is a core part of the desktop's event handling — stopping it may affect other bread apps until it's restarted.`)) {
				return;
			}
		}
		run(active ? "Stop" : "Start");
	}

	function openLogs() {
		invoke("open_logs", { unit });
	}

	onMount(refresh);
</script>

<Group title="Service">
	<Row label={unit}>
		<span class="dim">{active ? "Running" : "Stopped"}</span>
	</Row>
	<Row label="Starts at login">
		<span class="dim">{enabled ? "Yes" : "No"}</span>
	</Row>
	<div class="buttons">
		<button disabled={busy} onclick={toggle}>{active ? "Stop" : "Start"}</button>
		<button disabled={busy} onclick={() => run("Restart")}>Restart</button>
		<button onclick={openLogs}>View logs</button>
	</div>
	{#if hasConfig}
		<Hint text="Save (below) only writes the config file — click Restart above for the running service to pick up the change." />
	{/if}
</Group>

<style>
	.dim {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.buttons {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-top: var(--space-xs, 4px);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}

	button:hover {
		background-color: color-mix(in srgb, var(--on-surface) 14%, transparent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
