<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
	import { snapshotLabel } from "$lib/snapshotLabel";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import { snapshotRestoreConfirmation } from "$lib/snapshotRestore";
	import Sheet from "$lib/components/Sheet.svelte";

	interface SnapshotRow {
		number: string;
		date: string;
		description: string;
		kind?: string;
	}
	interface SnapshotAvailability {
		state: "available" | "no-config" | "live-session" | "error";
		live: boolean;
		error: string | null;
	}

	let snapshots = $state<SnapshotRow[] | "loading">("loading");
	let availability = $state<SnapshotAvailability | "loading">("loading");
	let errorHint = $state<string | null>(null);
	let picked = $state<SnapshotRow | null>(null);
	let sheetError = $state("");
	let showAll = $state(false);
	let restoring = $state(false);
 async function takeSnapshot(){if(restoring)return;restoring=true;try{await invoke("create_manual_snapshot");await refresh()}catch(e){errorHint=String(e)}finally{restoring=false}}

	async function refresh() {
		errorHint = null;
		availability = "loading";
		picked = null;
		try {
			const a = await invoke<SnapshotAvailability>("get_snapshot_availability");
			availability = a;
			if (a.state !== "available") {
				snapshots = [];
				return;
			}
		} catch (e) {
			availability = { state: "error", live: false, error: `${e}` };
			snapshots = [];
			return;
		}
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
	}

	let groupHint = $derived(
		availability === "loading"
			? "BOS saves the system before every update. If something breaks, go back to one of these."
			: availability.state === "live-session"
				? "Snapshots aren't available in a live session. They start after you install BOS."
				: availability.state === "no-config"
					? "Snapshots aren't set up on this system yet."
					: "BOS saves the system before every update. If something breaks, go back to one of these.",
	);

	onMount(refresh);

	// "2026-10-07 16:32:00" -> "7 Oct, 16:32" (or "Today, 16:32").
	function when(date: string): string {
		const d = new Date(date.replace(" ", "T"));
		if (Number.isNaN(d.getTime())) return date;
		const time = d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
		if (d.toDateString() === new Date().toDateString()) return `Today, ${time}`;
		return `${d.toLocaleDateString(undefined, { day: "numeric", month: "short" })}, ${time}`;
	}

	async function deleteSnapshot(number: string) {
		if (!(await confirmAction({ title: `Delete snapshot #${number}?`, message: "This can't be undone.", confirmLabel: "Delete" }))) return;
		sheetError = "";
		try {
			await invoke("delete_snapshot", { number });
			await refresh();
		} catch (e) {
			sheetError = `Couldn't delete it: ${e}`;
		}
	}

	async function restore(number: string) {
		if (restoring) return;
		restoring = true;
		sheetError = "";
		try {
			await invoke("restore_snapshot", { number });
		} catch (e) {
			sheetError = `Couldn't restore: ${e}`;
		} finally {
			restoring = false;
		}
	}
</script>

<ViewScaffold title="Snapshots">
	<Group title="System snapshots" hint={groupHint}>
		{#if availability === "loading"}
			<p class="muted">Loading…</p>
		{:else if availability.state === "live-session"}
			<p class="muted">Snapshots are not available in a live session. They start after you install BOS.</p>
		{:else if availability.state === "no-config"}
			<p class="muted">Snapshots aren't set up on this system yet. They start after you install BOS.</p>
		{:else if availability.state === "error"}
			<p class="muted bad">Couldn't read snapshots. {availability.error}</p>
		{:else if snapshots === "loading"}
			<p class="muted">Loading…</p>
		{:else if errorHint}
			<p class="muted bad">Couldn't read snapshots. {errorHint}</p>
		{:else if snapshots.length === 0}
			<p class="muted">No snapshots yet. One is taken before every update.</p>
		{:else}
			<div class="timeline">
				<div class="event current"><div class="dot"></div><div class="lbl"><b>Now</b><small>Current system</small></div></div>
				{#each [...snapshots].reverse().slice(0, showAll ? undefined : 5) as snap (snap.number)}
					<div class="event">
						<div class="dot"></div>
						<div class="lbl"><b title={snap.description}>{snapshotLabel(snap.description, snap.kind) || `Snapshot ${snap.number}`}</b><small>{when(snap.date)} · #{snap.number}</small></div>
						<button class="btn small" disabled={restoring} onclick={() => (picked = snap)}>Restore…</button>
					</div>
				{/each}
			</div>
			{#if snapshots.length > 5 && !showAll}<button class="btn small ghost older" onclick={() => (showAll = true)}>Show {snapshots.length - 5} older</button>{/if}
		{/if}
		{#if typeof availability !== "string" && availability.state === "available"}
			<details class="ui-disclosure">
				<summary>More</summary>
				<div class="ui-item">
					<div class="lb"><b>Take a snapshot now</b><small>Handy before you change something by hand.</small></div>
					<button class="btn small" disabled={restoring} onclick={takeSnapshot}>Take snapshot</button>
				</div>
				<div class="ui-item">
					<div class="lb"><b>How many are kept</b><small>Up to 20 automatic snapshots and 10 important ones, fewer when the disk is getting full.</small></div>
				</div>
			</details>
		{/if}
	</Group>
</ViewScaffold>

{#if picked}
	{@const snap = picked}
	<Sheet title="Restore snapshot #{snap.number}?" lede={snapshotRestoreConfirmation(snap.number)} onClose={() => (picked = null)}>
		<p class="muted">{snapshotLabel(snap.description, snap.kind) || snap.description} · {when(snap.date)}</p>
		{#if sheetError}<p class="muted bad" role="alert">{sheetError}</p>{/if}
		{#snippet actions()}
			<button class="btn remove" disabled={restoring} onclick={() => deleteSnapshot(snap.number)}>Delete snapshot</button>
			<span class="spacer"></span>
			<button class="btn" onclick={() => (picked = null)}>Cancel</button>
			<button class="btn primary" disabled={restoring} onclick={() => restore(snap.number)}>{restoring ? "Restoring…" : "Restore and restart"}</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; line-height:1.45; } .bad { color:var(--red); }
	.timeline { display:flex; flex-direction:column; padding:4px 0; }
	.event { display:flex; align-items:center; gap:12px; padding:10px 0; position:relative; }
	.event::before { content:""; position:absolute; left:5px; top:-10px; height:20px; width:2px; background:var(--line); }
	.event:first-child::before { display:none; }
	.dot { width:12px; height:12px; border-radius:50%; background:var(--surface-2); border:2px solid var(--muted); flex:none; position:relative; z-index:1; }
	.current .dot { background:var(--accent); border-color:var(--accent); }
	.lbl { flex:1; min-width:0; }
	.lbl b { display:block; font-weight:400; overflow-wrap:anywhere; }
	.lbl small { display:block; color:var(--muted); font-size:12px; margin-top:2px; }
	.spacer { flex:1; }
	.older { margin:0 0 8px 22px; }
</style>
