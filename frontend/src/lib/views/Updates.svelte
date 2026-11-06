<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
	import { updateRun } from "$lib/updateRun.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import FirmwareDeviceList from "$lib/components/FirmwareDeviceList.svelte";
	import { go } from "$lib/nav.svelte";
	import Download from "@lucide/svelte/icons/download";
	import Cpu from "@lucide/svelte/icons/cpu";
	import Check from "@lucide/svelte/icons/check";
	import CircleAlert from "@lucide/svelte/icons/circle-alert";
	import { heroState, failedParts } from "$lib/updateHero";
	import { updateCounts } from "$lib/updateCount";

	interface PendingUpdate {
		name: string;
		current: string;
		latest: string;
	}
	interface FwDevice {
		name: string;
		version: string;
		security_database?: boolean;
	}
	interface NvidiaOffer {
		gpu: string;
		reason: string;
		packages: string[];
		installed: boolean;
	}
	interface UpdatesStatus {
		total: number;
		pacman: PendingUpdate[];
		pacman_error: string | null;
		bakery: PendingUpdate[];
		bakery_error: string | null;
		firmware: FwDevice[];
		nvidia: NvidiaOffer | null;
	}

	let status = $state<UpdatesStatus | null>(null);
	let counts = $derived(updateCounts(status));
	let total = $derived(counts.total);
	let hero = $derived(heroState(status, updateRun.busy));
	let summary = $derived.by(() => {
		if (!status) return "";
		const parts: string[] = [];
		if (counts.system) parts.push(`${counts.system} system ${counts.system === 1 ? "package" : "packages"}`);
		if (counts.bread) parts.push(`${counts.bread} bread ${counts.bread === 1 ? "app" : "apps"}`);
		const kernel = status.pacman.some((p) => p.name === "linux" || p.name.startsWith("linux-")) ? " · includes a new kernel, restart after" : "";
		const firmware = counts.firmware ? ` ${counts.firmware} firmware ${counts.firmware === 1 ? "device" : "devices"} can be checked on the Firmware page.` : "";
		if (!parts.length) return `${firmware.trim()}${kernel}`.trim();
		return `${parts.join(" and ")} · a safety snapshot is taken first${kernel}.${firmware}`;
	});
	// "0.9.1-dev.20261002094801+63126c4" -> "0.9.1 (dev build)".
	function short(v: string): string {
		const m = v.match(/^([\d.]+)(?:-(dev|beta|rc)[.\d]*)?/);
		return m ? `Version ${m[1]}${m[2] ? ` (${m[2]} build)` : ""}` : v;
	}
	async function recheck() {
		status = null;
		await refresh();
	}


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
  updateRun.steps = [{label:"Taking a safety snapshot",state:"running"}, ...jobs.map(j => ({label:j.label,state:"waiting" as const}))];
  let index = 0;
  try {
   const snapshot = await invoke<string|null>("create_update_snapshot");
   if(!snapshot && !(await confirmAction({title:"Update without a snapshot?",message:"No root snapshot configuration was found. Continue updating without a system snapshot?",confirmLabel:"Update anyway"}))) { updateRun.message = "Update cancelled."; updateRun.steps[0].state = "failed"; return; }
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
	<div class="hero-card">
		<span class="big">{#if hero === "current"}<Check size={26} />{:else if hero === "failed"}<CircleAlert size={26} />{:else}<Download size={26} />{/if}</span>
		<div>
			{#if hero === "updating"}
				<h3>Updating…</h3><p>You can keep working. Don't turn the computer off.</p>
			{:else if hero === "checking"}
				<h3>Checking for updates…</h3><p>This takes a few seconds.</p>
			{:else if hero === "failed" && status}
				<h3>Couldn't check for updates</h3><p>BOS couldn't check {failedParts(status)}. Check your connection and try again. Details are below.</p>
			{:else if hero === "firmware"}
				<h3>No package updates</h3><p>{counts.firmware} firmware {counts.firmware === 1 ? "device can" : "devices can"} be checked on the Firmware page.</p>
			{:else if hero === "current"}
				<h3>You're up to date</h3><p>{updateRun.message || "System and bread apps are current."}</p>
			{:else}
				<h3>{total} {total === 1 ? "update" : "updates"} ready</h3><p>{summary}</p>
			{/if}
		</div>
		<div class="acts">
			{#if status && (total > 0 || counts.firmware > 0)}<button class="btn primary" disabled={updateRun.busy} onclick={updateEverything}>{updateRun.busy ? "Updating…" : "Update everything"}</button>{/if}
			{#if status && !updateRun.busy}<button class="btn ghost small" onclick={recheck}>Check again</button>{/if}
		</div>
	</div>
	{#if updateRun.steps.length}
		<Group title="Progress">
			<ol class="steps" aria-live="polite">
				{#each updateRun.steps as step (step.label)}<li class={step.state}>{step.label}</li>{/each}
			</ol>
			{#if updateRun.message}<p class="msg" class:bad={updateRun.steps.some((st) => st.state === "failed")} role="status">{updateRun.message}</p>{/if}
		</Group>
	{:else if updateRun.message && total > 0}
		<p class="msg" role="status">{updateRun.message}</p>
	{/if}

	{#if status?.nvidia}
		<Group title="Graphics driver">
			<div class="ui-item">
				<span class="ico"><Cpu size={18} /></span>
				<div class="lb"><b>{status.nvidia.gpu}</b><small>{status.nvidia.installed ? "The NVIDIA driver is installed. Restart to use it." : `${status.nvidia.reason} Installs the NVIDIA driver; restart afterwards.`}</small></div>
				<button class="btn small" class:primary={!status.nvidia.installed} disabled={updateRun.busy} onclick={() => run("nvidia_setup")}>{status.nvidia.installed ? "Set up again" : "Install driver"}</button>
			</div>
		</Group>
	{/if}

	{#if status && (status.pacman.length || status.bakery.length || status.firmware.length || status.pacman_error || status.bakery_error)}
		<Group title="What will update">
			{#if status.pacman_error}<p class="msg bad">Couldn't check system updates: {status.pacman_error}</p>{/if}
			{#if status.bakery_error}<p class="msg bad">Couldn't check bread app updates: {status.bakery_error}</p>{/if}
			{#if status.bakery.length}
				<details class="ui-disclosure" open={status.bakery.length <= 6}>
					<summary>Bread apps ({status.bakery.length})</summary>
					{#each status.bakery as pkg (pkg.name)}
						<div class="ui-item">
							<div class="lb"><b>{pkg.name}</b><small>{short(pkg.latest)}</small></div>
							<button class="btn small ghost" disabled={updateRun.busy} onclick={() => run("bakery_update", { name: pkg.name })}>Update</button>
						</div>
					{/each}
				</details>
			{/if}
			{#if status.pacman.length}
				<details class="ui-disclosure">
					<summary>System packages ({status.pacman.length}){status.pacman.some((p) => p.name === "linux" || p.name.startsWith("linux-")) ? ", including a new kernel" : ""}</summary>
					<div class="pkgs">
						{#each status.pacman as pkg (pkg.name)}
							<div class="pkg"><span>{pkg.name}</span><small>{pkg.latest}</small></div>
						{/each}
					</div>
				</details>
			{/if}
			{#if status.firmware.length}
				<details class="ui-disclosure">
					<summary>Firmware devices ({status.firmware.length})</summary>
					<FirmwareDeviceList devices={status.firmware} />
					<div class="foot"><button class="btn small" disabled={updateRun.busy} onclick={() => go("firmware")}>Open Firmware</button></div>
				</details>
			{/if}
			<details class="ui-disclosure">
				<summary>Update one part only</summary>
				<div class="foot">
					<button class="btn small" disabled={updateRun.busy || !status.pacman.length} onclick={() => run("pacman_system_update")}>System packages</button>
					<button class="btn small" disabled={updateRun.busy || !status.bakery.length} onclick={() => run("bakery_update_all")}>Bread apps</button>
					<button class="btn small" disabled={updateRun.busy || !status.firmware.length} onclick={() => go("firmware")}>Firmware</button>
				</div>
			</details>
		</Group>
	{/if}

	<Group title="If an update breaks something">
		<div class="ui-item">
			<div class="lb"><b>Go back to before the update</b><small>Every update takes a snapshot first. Restart and pick it from the boot menu, or choose it in Backup & restore.</small></div>
			<button class="btn small" onclick={() => go("restore")}>Open</button>
		</div>
	</Group>

	{#if updateRun.log.length}
		<details class="ui-disclosure"><summary>Details</summary><LogView lines={updateRun.log} /></details>
	{/if}
</ViewScaffold>

<style>
	.steps { list-style:none; margin:4px 0 10px; padding:0; display:flex; flex-direction:column; gap:8px; font-size:13px; }
	.steps li { display:flex; align-items:center; gap:10px; color:var(--muted); }
	.steps li::before { content:""; width:10px; height:10px; border-radius:50%; border:2px solid var(--line); flex:none; }
	.steps li.running { color:var(--fg); } .steps li.running::before { border-color:var(--accent); border-top-color:transparent; animation:spin 0.9s linear infinite; }
	.steps li.done::before { background:var(--accent); border-color:var(--accent); }
	.steps li.failed { color:var(--red); } .steps li.failed::before { border-color:var(--red); }
	@keyframes spin { to { transform:rotate(360deg); } }
	.msg { margin:4px 2px 10px; font-size:12.5px; color:var(--muted); } .msg.bad { color:var(--red); }
	.pkgs { columns:2 220px; column-gap:16px; padding:2px 2px 10px; max-height:50vh; overflow:auto; }
	.pkg { break-inside:avoid; display:flex; justify-content:space-between; gap:8px; font-size:12.5px; padding:3px 0; }
	.pkg small { color:var(--faint); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
	.foot { display:flex; gap:8px; flex-wrap:wrap; padding:2px 0 10px; }
</style>
