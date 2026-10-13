<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface AutostartEntry {
		command: string;
		label: string;
		enabled: boolean;
	}

	const BREADHELP_CMD = "breadhelp --autostart";
	const BREADHELP_LABEL = "BOS Help (first-run onboarding)";

	let entries = $state<AutostartEntry[] | null>(null);
	let status = $state("");

	let helpEntry = $derived(entries?.find((e) => e.command.trim().startsWith("breadhelp")) ?? null);
	let autostartOn = $derived(helpEntry?.enabled ?? false);

	onMount(async () => {
		entries = await invoke<AutostartEntry[]>("get_autostart_entries");
	});

	async function setAutostart(enabled: boolean) {
		if (!entries) return;
		const next = entries.map((e) => ({ ...e }));
		const i = next.findIndex((e) => e.command.trim().startsWith("breadhelp"));
		if (i >= 0) {
			next[i].enabled = enabled;
		} else if (enabled) {
			next.push({ command: BREADHELP_CMD, label: BREADHELP_LABEL, enabled: true });
		}
		try {
			await invoke("save_autostart_entries", { entries: next, allowSystemChanges: true });
			entries = next;
			status = enabled ? "The welcome guide opens at your next login." : "The welcome guide won't open at login.";
		} catch (e) {
			status = `Error: ${e}`;
		}
	}
</script>

<ViewScaffold title="Help">
	<Group title="Help">
		<div class="ui-item">
			<div class="lb"><b>BOS Help</b><small>Guides for getting started and fixing common problems.</small></div>
			<button class="btn small" onclick={() => invoke("open_breadhelp")}>Open</button>
		</div>
		<SwitchField label="Show the welcome guide at login" bind:value={() => autostartOn, (v) => setAutostart(v)} />
		{#if status}<p class="msg" role="status">{status}</p>{/if}
	</Group>
</ViewScaffold>

<style>
	.msg { margin:0 2px 10px; color:var(--muted); font-size:12px; }
</style>
