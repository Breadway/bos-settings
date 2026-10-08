<script lang="ts">
	import { updateRun } from "$lib/updateRun.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import { go } from "$lib/nav.svelte";
	import Download from "@lucide/svelte/icons/download";
	import Cpu from "@lucide/svelte/icons/cpu";

	interface PendingUpdate {
		name: string;
		current: string;
		latest: string;
	}
	interface FwDevice {
		name: string;
		version: string;
	}
	interface NvidiaOffer {
		gpu: string;
		reason: string;
		packages: string[];
		installed: boolean;
	}
	interface UpdatesStatus {
		pacman: PendingUpdate[];
		pacman_error: string | null;
		bakery: PendingUpdate[];
		bakery_error: string | null;
		firmware: FwDevice[];
		nvidia: NvidiaOffer | null;
	}

	let status = $state<UpdatesStatus | null>(null);


	async function refresh() {
		status = await invoke<UpdatesStatus>("get_updates_status");
	}

	onMount(refresh);

	function appendLine(line: string) {
		updateRun.log = [...updateRun.log, line];
	}

	async function run(command: string, args: Record<string, unknown> = {}) {
		if (updateRun.busy) return;
		updateRun.log = [];
		updateRun.steps = [];
		updateRun.message = "";
		updateRun.busy = true;
		try { if (!await runStreamed(command, args, appendLine)) updateRun.message = "The operation did not finish. See the log for details."; await refresh(); } catch(e) { updateRun.message = String(e); } finally { updateRun.busy = false; }
	}

 async function updateEverything() {
  if (updateRun.busy || !status) return;
  const jobs = [{label:"System packages", command:"pacman_system_update"},{label:"Bread apps",command:"bakery_update_all"},{label:"Firmware",command:"fwupd_update"}];
  updateRun.log = []; updateRun.message = ""; updateRun.busy = true;
  updateRun.steps = [{label:"Safety snapshot",state:"running"}, ...jobs.map(j => ({label:j.label,state:"waiting" as const}))];
  let index = 0;
  try {
   const snapshot = await invoke<string|null>("create_update_snapshot");
   if(!snapshot && !confirm("No root snapshot configuration was found. Continue updating without a system snapshot?")) { updateRun.message = "Update cancelled."; updateRun.steps[0].state = "failed"; return; }
   updateRun.log.push(snapshot ? `Created system snapshot ${snapshot}` : "Continuing without a system snapshot");
   updateRun.steps[0].state = "done";
   for (const [i,job] of jobs.entries()) {
    index = i + 1; updateRun.steps[index].state = "running";
    if (!await runStreamed(job.command, {}, appendLine)) throw new Error(`${job.label} did not finish. Review the log before retrying.`);
    updateRun.steps[index].state = "done";
   }
   updateRun.message = "Updates complete. Firmware may require a restart.";
   await refresh();
  } catch(e) { updateRun.steps[index].state = "failed"; updateRun.message = String(e); }
  finally { updateRun.busy = false; }
 }
</script>

<ViewScaffold title="Updates">
 <Group title="Update everything" hint="Create a system snapshot, then update system packages, bread apps and firmware. Authentication may be requested.">
  <button class="btn primary" disabled={updateRun.busy || !status} onclick={updateEverything}>{updateRun.busy ? "Updating…" : "Update everything"}</button>
  {#if updateRun.steps.length}<ol class="steps" aria-live="polite">{#each updateRun.steps as step}<li class:failed={step.state === "failed"}>{step.label}: {step.state}</li>{/each}</ol>{/if}
  {#if updateRun.message}<p role="status">{updateRun.message}</p>{/if}
 </Group>
	{#if status?.nvidia}
		<Group title="NVIDIA driver" wide>
			<div class="offer">
				<Cpu size={20} />
				<div class="offer-text">
					<strong>{status.nvidia.gpu}</strong>
					<p>{status.nvidia.reason}</p>
					{#if status.nvidia.installed}
						<Hint text="Driver and Hyprland env drop-in are in place. Reboot to start a working session." />
					{:else}
						<Hint text="Installs nvidia + nvidia-utils (not cuda) and writes ~/.config/hypr/nvidia.lua. hyprland.lua loads that file only if it exists. Reboot after." />
					{/if}
				</div>
				<button
					class="primary"
					disabled={updateRun.busy}
					onclick={() => run("nvidia_setup")}
				>
					{status.nvidia.installed ? "Re-run setup" : "Install driver"}
				</button>
			</div>
		</Group>
	{/if}

	<Group
		title="System packages (pacman)"
		hint="Official repositories. Authentication is required."
		wide
	>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.pacman_error}
			<Hint text={status.pacman_error} />
		{:else if status.pacman.length === 0}
			<EmptyState icon={Download} title="Pacman is up to date" hint="No pending official-repo upgrades." />
		{:else}
			<div class="list">
				{#each status.pacman as pkg (pkg.name)}
					<div class="row">
						<span class="name">{pkg.name}</span>
						<span class="version">{pkg.current} → {pkg.latest}</span>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button disabled={updateRun.busy} class="primary" onclick={() => run("pacman_system_update")}>Update system</button>
			<button disabled={updateRun.busy} onclick={refresh}>Refresh</button>
		</div>
	</Group>

	<Group
		title="Bread ecosystem (bakery)"
		hint="Bakery packages waiting to update."
		wide
	>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.bakery_error}
			<Hint text={status.bakery_error} />
		{:else if status.bakery.length === 0}
			<EmptyState icon={Download} title="Bakery packages are current" hint="Nothing on this track wants an update." />
		{:else}
			<div class="list">
				{#each status.bakery as pkg (pkg.name)}
					<div class="row">
						<span class="name">{pkg.name}</span>
						<span class="version">{pkg.current ? `${pkg.current} → ` : ""}{pkg.latest}</span>
						<button disabled={updateRun.busy} onclick={() => run("bakery_update", { name: pkg.name })}>Update</button>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button disabled={updateRun.busy} class="primary" onclick={() => run("bakery_update_all")}>Update all bakery</button>
		</div>
	</Group>

	<Group title="Firmware (fwupd)" hint="Same list as the Firmware page. fwupd-refresh.timer already refreshes metadata in the background." wide>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.firmware.length === 0}
			<EmptyState icon={Download} title="No updatable firmware" hint="Not every device speaks fwupd." />
		{:else}
			<div class="list">
				{#each status.firmware as dev, i (`${i}:${dev.name}`)}
					<div class="row">
						<span class="name">{dev.name}</span>
						<span class="version">{dev.version}</span>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button disabled={updateRun.busy} onclick={() => run("fwupd_refresh")}>Check for updates</button>
			<button disabled={updateRun.busy} class="primary" onclick={() => run("fwupd_update")}>Update firmware</button>
		</div>
	</Group>

	<Group
		title="Rollback"
		hint="Boot a snapshot from GRUB (BOS snapshots)."
	>
		<Hint text="Pick the snapshot in GRUB. bakery rollback undoes one package." />
		<div class="btn-row">
			<button onclick={() => go("snapshots")}>Open Snapshots</button>
		</div>
	</Group>

	<LogView lines={updateRun.log} />
</ViewScaffold>

<style>
	.offer {
		display: flex;
		align-items: flex-start;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px);
	}

	.offer-text {
		flex: 1;
		min-width: 0;
	}

	.offer-text p {
		margin: 4px 0;
		opacity: 0.8;
	}

	.list {
		max-height: 240px;
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
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
		min-width: 0;
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.version {
		opacity: 0.75;
		font-size: var(--font-size-secondary, 12px);
		flex-shrink: 0;
	}

	.btn-row {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-top: var(--space-sm, 8px);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.steps { color:var(--muted); font-size:12px; padding-left:20px; } .failed { color:var(--red); }
</style>
