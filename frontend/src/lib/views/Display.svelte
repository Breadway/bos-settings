<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	interface LiveMonitor {
		name: string;
		mode: string;
	}
	interface MonitorRule {
		output: string;
		mode: string;
		position: string;
		scale: string;
	}

	let liveMonitors = $state<LiveMonitor[] | null>(null);
	let rules = $state<MonitorRule[] | null>(null);

	onMount(async () => {
		liveMonitors = await invoke<LiveMonitor[]>("get_live_monitors");
		rules = await invoke<MonitorRule[]>("get_monitor_rules");
	});

	function addRule() {
		rules = [...(rules ?? []), { output: "", mode: "preferred", position: "auto", scale: "auto" }];
	}

	function removeRule(i: number) {
		rules = rules!.filter((_, idx) => idx !== i);
		if (rules.length === 0) {
			rules = [{ output: "", mode: "preferred", position: "auto", scale: "auto" }];
		}
	}

	async function save() {
		await invoke("save_monitor_rules", { rules });
	}
</script>

<ViewScaffold title="Display">
	<Group title="Connected monitors">
		{#if liveMonitors && liveMonitors.length > 0}
			{#each liveMonitors as m (m.name)}
				<InfoRow label={m.name} value={m.mode} />
			{/each}
		{:else}
			<Hint text="No monitors detected (is Hyprland running?)" />
		{/if}
	</Group>

	<Group title="Advanced">
		<button class="secondary" onclick={() => invoke("open_hyprland_conf")}>Open hyprland.lua in editor</button>
		<button class="secondary" onclick={() => invoke("open_keybinds_viewer")}>View keybinds (breadhelp)</button>
	</Group>

	{#if rules}
		<Group
			title="Layout"
			hint="One row per monitor rule. Leave Output blank to match any monitor (the default — works on any hardware). Applies on next login/reload."
			wide
		>
			{#each rules as rule, i (i)}
				<div class="rule-row">
					<label>Output <input type="text" bind:value={rule.output} placeholder="any (blank = all)" class="w-output" /></label>
					<label>Mode <input type="text" bind:value={rule.mode} placeholder="preferred / 1920x1080@60" class="w-mode" /></label>
					<label>Position <input type="text" bind:value={rule.position} placeholder="auto / 0x0" class="w-position" /></label>
					<label>Scale <input type="text" bind:value={rule.scale} placeholder="auto / 1" class="w-scale" /></label>
					<button class="remove" onclick={() => removeRule(i)}>Remove</button>
				</div>
			{/each}
			<button class="add" onclick={addRule}>Add monitor rule</button>

			<SaveButton onSave={save} />
		</Group>
	{/if}
</ViewScaffold>

<style>
	.rule-row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		margin-bottom: var(--space-xs, 4px);
		flex-wrap: wrap;
	}

	.rule-row label {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: var(--font-size-secondary, 12px);
	}

	.w-output {
		width: 12ch;
	}
	.w-mode {
		width: 18ch;
	}
	.w-position {
		width: 10ch;
	}
	.w-scale {
		width: 8ch;
	}

	input[type="text"] {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		cursor: pointer;
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
	}

	.add {
		background-color: var(--surface);
		color: var(--on-surface);
		margin-top: var(--space-sm, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		align-self: flex-start;
	}

	.secondary {
		background-color: var(--bg);
		color: var(--on-surface);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		align-self: flex-start;
		margin-bottom: var(--space-xs, 4px);
	}
</style>
