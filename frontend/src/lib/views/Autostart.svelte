<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import FolderOpen from "@lucide/svelte/icons/folder-open";

	interface AutostartEntry {
		command: string;
		label: string;
		enabled: boolean;
	}

	let entries = $state<AutostartEntry[] | null>(null);

	onMount(async () => {
		entries = await invoke<AutostartEntry[]>("get_autostart_entries");
	});

	function addEntry() {
		entries = [...(entries ?? []), { command: "", label: "", enabled: true }];
	}

	function removeEntry(i: number) {
		entries = entries!.filter((_, idx) => idx !== i);
	}

	async function browseFor(entry: AutostartEntry) {
		const picked = await open({ directory: false, defaultPath: "/usr/bin" });
		if (typeof picked === "string") entry.command = picked;
	}

	async function save() {
		await invoke("save_autostart_entries", { entries });
	}
</script>

<ViewScaffold title="Startup Apps">
	{#if entries}
		<Group
			title="Extra autostart apps"
			hint="Extra apps after login. Bar and clipboard always start."
			wide
		>
			{#each entries as entry, i (i)}
				<div class="row">
					<button
						class="switch"
						class:on={entry.enabled}
						role="switch"
						aria-checked={entry.enabled}
						aria-label="Enabled"
						onclick={() => (entry.enabled = !entry.enabled)}
					>
						<span class="knob"></span>
					</button>
					<div class="fields">
						<input type="text" bind:value={entry.label} placeholder="Label" class="label" title={entry.label} />
						<input type="text" bind:value={entry.command} placeholder="command" class="command" title={entry.command} />
					</div>
					<button class="browse" onclick={() => browseFor(entry)} aria-label="Browse for program">
						<FolderOpen size={14} />
					</button>
					<button class="remove" onclick={() => removeEntry(i)}>Remove</button>
				</div>
			{/each}
			<button class="add" onclick={addEntry}>Add app</button>

			<SaveButton onSave={save} />
		</Group>
	{/if}
</ViewScaffold>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		margin-bottom: var(--space-xs, 4px);
	}

	.fields {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.label,
	.command {
		width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.command {
		opacity: 0.65;
		font-size: var(--font-size-secondary, 12px);
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

	.browse {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
		display: flex;
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
	}

	.switch {
		width: 36px;
		height: 20px;
		flex-shrink: 0;
		border-radius: 999px;
		background-color: var(--overlay);
		padding: 2px;
		display: flex;
		align-items: center;
	}

	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}

	.knob {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}
</style>
