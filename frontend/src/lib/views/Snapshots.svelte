<script lang="ts">
	import { snapshotLabel } from "$lib/snapshotLabel";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import History from "@lucide/svelte/icons/history";
	import CircleAlert from "@lucide/svelte/icons/circle-alert";
	import { confirmAndRestoreSnapshot } from "$lib/snapshotRestore";

	interface SnapshotRow {
		number: string;
		date: string;
		description: string;
		kind?: string;
	}

	let snapshots = $state<SnapshotRow[] | "loading">("loading");
	let errorHint = $state<string | null>(null);
	let selected = $state<string | null>(null);
	let restoring = $state(false);
 async function takeSnapshot(){if(restoring)return;restoring=true;try{await invoke("create_manual_snapshot");await refresh()}catch(e){errorHint=String(e)}finally{restoring=false}}

	async function refresh() {
		errorHint = null;
		try {
			snapshots = await invoke<SnapshotRow[]>("get_snapshots");
		} catch (e) {
			const msg = `${e}`.toLowerCase();
			if (msg.includes("no permission")) {
				errorHint = "This user is not in ALLOW_USERS for snapper.";
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
				`Reboot to pick snapshot #${selected} in GRUB? BOS boots snapshots from the GRUB “BOS snapshots” submenu. This does not run snapper rollback.`,
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

	async function restoreSelected() {
		if (!selected || restoring) return;
		restoring = true;
		try {
			await confirmAndRestoreSnapshot(
				selected,
				(message) => confirm(message),
				(number) => invoke("restore_snapshot", { number }),
				(message) => alert(message),
			);
		} finally {
			restoring = false;
		}
	}
</script>

<ViewScaffold title="Snapshots">
 <button class="btn" disabled={restoring} onclick={takeSnapshot}>Take a snapshot now</button>
	<Hint text="Reboot and pick a snapshot in GRUB to undo an update." />
	<Hint text="Automatic cleanup keeps 2 to 20 regular snapshots and 4 to 10 important snapshots. With Btrfs quotas enabled, it also keeps at least 20% of the filesystem free and limits snapshots to 50% of its space. Low disk space can reduce these counts." />
	<Group
		title="What to pick in GRUB"
		hint="Reboot and choose that entry in GRUB."
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
                <div class="timeline">
                <div class="event current"><div class="dot"></div><div><b>Now</b><small>Current system</small></div></div>
                {#each snapshots as snap (snap.number)}
                 <div class="event" class:selected={selected===snap.number}>
                  <button class="dot" aria-label={`Select snapshot ${snap.number}`} onclick={()=>selected=snap.number}></button>
                  <button class="snapshot-label" onclick={()=>selected=snap.number}><b title={snap.description}>{snapshotLabel(snap.description, snap.kind) || `Snapshot ${snap.number}`}</b><small>{snap.date} · #{snap.number}</small></button>
                  <button class="btn" disabled={restoring} onclick={async()=>{selected=snap.number;await restoreSelected()}}>Restore…</button>
                 </div>
                {/each}
                </div>
			{/if}
		</div>

		<div class="btn-row">
			<button onclick={refresh}>Refresh</button>
			<button disabled={!selected} onclick={bootIntoSelected}>Reboot to pick in GRUB</button>
			<button class="destructive" disabled={!selected || restoring} onclick={restoreSelected}>
				{restoring ? "Restoring…" : "Restore selected snapshot and reboot"}
			</button>
			<button class="destructive" disabled={!selected} onclick={deleteSelected}>Delete selected</button>
		</div>
	</Group>
</ViewScaffold>

<style>
 .timeline{display:flex;flex-direction:column;padding:8px 0}.event{display:flex;align-items:center;gap:12px;padding:12px 0;position:relative}.dot{width:12px;height:12px;border-radius:50%;background:var(--surface-2);border:2px solid var(--muted);padding:0;flex:none}.current .dot,.selected .dot{background:var(--accent);border-color:var(--accent)}.snapshot-label{flex:1;min-width:0;display:flex;flex-direction:column;text-align:left;background:transparent;border:0;padding:0;color:var(--fg)}.event b{font-weight:400}.event small{display:block;color:var(--muted);font-size:12px;margin-top:2px}.event:before{content:"";position:absolute;left:5px;top:-3px;height:14px;width:2px;background:var(--line)}.event:first-child:before{display:none}
	.list {

		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}








	.btn-row {
		display: flex;
		flex-wrap: wrap;
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
