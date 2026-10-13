<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Group from "./Group.svelte";

	// One background service: what it is, whether it is running, and
	// start/stop/restart/logs. `name` is the plain-language name; the unit
	// file name only shows as a tooltip.
	let {
		unit,
		name,
		about = "",
		critical = false,
		hasConfig = false,
		bare = false,
	}: { unit: string; name?: string; about?: string; critical?: boolean; hasConfig?: boolean; bare?: boolean } = $props();

	let active = $state<boolean | null>(null);
	let enabled = $state(false);
	let busy = $state(false);
	let error = $state("");

	async function refresh() {
		const status = await invoke<{ active: boolean; enabled: boolean }>("get_service_status", { unit });
		active = status.active;
		enabled = status.enabled;
	}

	async function run(action: "Start" | "Stop" | "Restart") {
		busy = true;
		error = "";
		try {
			await invoke("service_action", { unit, action });
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
			await refresh();
		}
	}

	function toggle() {
		if (active && critical && !confirm(`Stop ${name ?? unit}? Other bread apps may stall until it is started again.`)) return;
		run(active ? "Stop" : "Start");
	}

	onMount(refresh);
	let summary = $derived(active === null ? "Checking…" : `${active ? "Running" : "Stopped"} · ${enabled ? "starts at login" : "doesn't start at login"}`);
</script>

{#snippet body()}
	<div class="svc" data-setting={name ?? unit}>
		<div class="lb">
			<b title={unit}>{name ?? unit}</b>
			<small class:off={active === false}>{summary}{about ? ` · ${about}` : ""}</small>
		</div>
		<div class="acts">
			<button class="btn small" disabled={busy || active === null} onclick={toggle}>{active ? "Stop" : "Start"}</button>
			{#if active}<button class="btn small" disabled={busy} onclick={() => run("Restart")}>Restart</button>{/if}
			<button class="btn small ghost" onclick={() => invoke("open_logs", { unit })}>Logs</button>
		</div>
	</div>
	{#if hasConfig}<p class="hint">Changes here save straight away. Restart it to pick them up.</p>{/if}
	{#if error}<p class="err" role="alert">{error}</p>{/if}
{/snippet}

{#if bare}
	{@render body()}
{:else}
	<Group title="Background service">{@render body()}</Group>
{/if}

<style>
	.svc { display:flex; align-items:center; gap:12px; padding:9px 2px; flex-wrap:wrap; }
	.lb { flex:1 1 200px; min-width:0; }
	.lb b { display:block; font-weight:400; }
	.lb small { display:block; color:var(--muted); font-size:12px; margin-top:2px; }
	.lb small.off { color:var(--red); }
	.acts { display:flex; gap:6px; flex-wrap:wrap; }
	.hint { margin:0 2px 10px; color:var(--muted); font-size:12px; }
	.err { margin:0 2px 10px; color:var(--red); font-size:12px; }
</style>
