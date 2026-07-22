<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import History from "@lucide/svelte/icons/history";
	import CircleAlert from "@lucide/svelte/icons/circle-alert";

	interface SnapshotRow {
		number: string;
		date: string;
		description: string;
	}

	let snapshots = $state<SnapshotRow[] | "loading">("loading");
	let errorHint = $state<string | null>(null);
	let selected = $state<string | null>(null);

	async function refresh() {
		errorHint = null;
		try {
			snapshots = await invoke<SnapshotRow[]>("get_snapshots");
		} catch (e) {
			const msg = `${e}`.toLowerCase();
			if (msg.includes("no permission")) {
				errorHint = "This user isn't allowed to run snapper. Check ALLOW_USERS in /etc/snapper/configs/root — it should list your username.";
			} else if (msg.includes("unknown config") || msg.includes("no such file")) {
				errorHint = "No snapper config exists for root yet, so nothing is being snapshotted. This should be set up automatically at install.";
			} else {
				errorHint = `${e}`;
			}
			snapshots = [];
		}
		selected = null;
	}

	onMount(refresh);

	function bootIntoSelected() {
		if (!selected) return;
		if (
			confirm(
				`Boot into snapshot #${selected}? Snapshots on BOS are booted directly from the GRUB menu (under "BOS snapshots"), not rolled back in place. Reboot now and pick this snapshot there.`,
			)
		) {
			invoke("reboot_system");
		}
	}

	async function deleteSelected() {
		if (!selected) return;
		if (!confirm(`Delete snapshot #${selected}? This cannot be undone.`)) return;
		try {
			await invoke("delete_snapshot", { number: selected });
			await refresh();
		} catch (e) {
			alert(`${e}`);
		}
	}
</script>

<ViewScaffold title="Snapshots">
	<Group
		title="System snapshots"
		hint="Created automatically by snap-pac on each pacman transaction. Boot into one from the GRUB menu to recover; delete old ones here."
		wide
	>
		<div class="list">
			{#if snapshots === "loading"}
				<Hint text="Loading…" />
			{:else if errorHint}
				<EmptyState icon={CircleAlert} title="Couldn't read snapshots" hint={errorHint} />
			{:else if snapshots.length === 0}
				<EmptyState icon={History} title="No snapshots yet" hint="Snapshots are created automatically on every pacman transaction." />
			{:else}
				{#each snapshots as snap (snap.number)}
					<button
						class="row"
						class:selected={selected === snap.number}
						onclick={() => (selected = snap.number)}
					>
						<span class="number">{snap.number}</span>
						<span class="date">{snap.date}</span>
						<span class="desc" title={snap.description}>{snap.description}</span>
					</button>
				{/each}
			{/if}
		</div>

		<div class="btn-row">
			<button onclick={refresh}>Refresh</button>
			<button disabled={!selected} onclick={bootIntoSelected}>Boot into selected…</button>
			<button class="destructive" disabled={!selected} onclick={deleteSelected}>Delete selected</button>
		</div>
	</Group>
</ViewScaffold>

<style>
	.list {
		max-height: 320px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		padding: var(--space-sm, 8px) var(--space-md, 12px);
		border-radius: var(--radius-primary, 8px);
		min-width: 0;
	}

	.row:hover {
		background-color: color-mix(in srgb, var(--surface), var(--on-surface) 8%);
	}

	.row.selected {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	.number {
		width: 4ch;
		flex-shrink: 0;
	}

	.date {
		width: 22ch;
		flex-shrink: 0;
		opacity: 0.85;
	}

	.desc {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.btn-row {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-top: var(--space-sm, 8px);
	}

	button {
		background-color: var(--bg);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	button.destructive {
		background-color: var(--red);
		color: var(--on-red);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
