<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
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
			await invoke("save_autostart_entries", { entries: next });
			entries = next;
			status = enabled ? "First-run help will launch at login." : "First-run help won't launch at login.";
		} catch (e) {
			status = `Error: ${e}`;
		}
	}
</script>

<ViewScaffold title="Help">
	<Group
		title="BOS Help"
		hint="breadhelp is the onboarding and help center — searchable guides, a keybind cheatsheet, a troubleshoot wizard, and a live tour. This panel just launches it; it doesn't copy the help app into Settings."
	>
		<button class="primary" onclick={() => invoke("open_breadhelp")}>Open breadhelp</button>
	</Group>

	<Group title="First-run autostart" hint="The extra autostart entry in hypr/autostart.json (breadhelp --autostart). Core desktop launch is separate and always runs.">
		<SwitchField label="Show help on login" bind:value={() => autostartOn, (v) => setAutostart(v)} />
		{#if !helpEntry}
			<Hint text="No breadhelp entry in autostart.json yet — turning this on adds the default first-run command." />
		{/if}
		{#if status}
			<Hint text={status} />
		{/if}
	</Group>
</ViewScaffold>

<style>
	.primary {
		align-self: flex-start;
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}
</style>
