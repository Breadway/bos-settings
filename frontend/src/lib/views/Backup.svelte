<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Info from "@lucide/svelte/icons/info";
	import FileField from "$lib/components/FileField.svelte";
	import PasswordField from "$lib/components/PasswordField.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface ResticSnapshot {
		id: string;
		time: string;
		paths: string[];
	}
	interface BackupStatus {
		restic_installed: boolean;
		repo: string;
		has_password: boolean;
		snapshots: ResticSnapshot[];
		error: string | null;
		home: string;
	}

	let schedule=$state("off");
	let originalSchedule="off";
	let scheduleReady=$state(false);
	onMount(async()=>{try{schedule=await invoke("get_backup_schedule");originalSchedule=schedule;}catch(e){message=String(e);}finally{scheduleReady=true;}});
	let st = $state<BackupStatus | null>(null);
	let repo = $state("");
	let password = $state("");
	let snapshots = $state<ResticSnapshot[]>([]);
	let selected = $state("latest");
	let restoreTarget = $state("");
	let lastAutoTarget = $state("");
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");
	let failed = $state(false);
	let restoring = $state(false);
	let kind = $derived(repo.startsWith("sftp:") ? "ssh" : "folder");

	// First backup to a new place: restic needs the store created first.
	async function backupNow() {
		message = ""; failed = false; log = [];
		if (!(await run("restic_backup"))) {
			const text = log.join("\n").toLowerCase();
			if (/does not exist|unable to open config|is there a repository/.test(text)) {
				if ((await run("restic_init")) && (await run("restic_backup"))) { message = "Backed up. Future backups only copy what changed."; return; }
			}
			failed = true;
			message = "The backup didn't finish. Open Details to see why.";
			return;
		}
		message = "Backed up.";
	}

	async function openRestore() {
		restoring = true;
		snapshots = [];
		await listSnaps();
		if (snapshots.length) selected = snapshots[snapshots.length - 1].id;
	}

	function defaultTarget(id: string): string {
		const home = st?.home ?? "";
		if (!home) return "";
		return `${home}/bos-restore-${id || "latest"}`;
	}

	$effect(() => {
		if (!st?.home) return;
		const auto = defaultTarget(selected || "latest");
		if (restoreTarget === "" || restoreTarget === lastAutoTarget) {
			if (restoreTarget !== auto) restoreTarget = auto;
			if (lastAutoTarget !== auto) lastAutoTarget = auto;
		}
	});

	async function refresh() {
		st = await invoke<BackupStatus>("get_backup_config");
		repo = st.repo;
	}

	onMount(refresh);

	async function save() {
		await invoke("save_backup_config", { input: { repo, password: password || null } });
		password = "";
		await refresh();
	}

	async function installRestic() {
		log = [];
		busy = true;
		await runStreamed("pacman_install", { packages: ["restic"] }, (line) => {
			log = [...log, line];
		});
		busy = false;
		await refresh();
	}

	async function run(command: string, args: Record<string, unknown> = {}): Promise<boolean> {
		busy = true;
		const ok = await runStreamed(command, args, (line) => {
			log = [...log, line];
		});
		busy = false;
		return ok !== false;
	}

	async function listSnaps() {
		message = "";
		try {
			snapshots = await invoke<ResticSnapshot[]>("list_restic_snapshots");
			if (snapshots.length === 0) message = "No snapshots in this repo yet.";
		} catch (e) {
			message = `${e}`;
			snapshots = [];
		}
	}

	async function restore(dryRun: boolean) {
		const snap = selected || "latest";
		const target = restoreTarget.trim() || defaultTarget(snap);
		const home = st?.home ?? "";
		if (!dryRun && home && (target === home || target === `${home}/`)) {
			message = "Pick a new folder, not your home folder itself, so nothing gets overwritten.";
			return;
		}
		if (!dryRun) {
			const ok = confirm(
				`Copy the files from this backup into ${target}?`,
			);
			if (!ok) return;
		}
		log = [];
		const ok = await run(dryRun ? "restic_restore_dry_run" : "restic_restore", {
			snapshot: snap,
			target,
		});
		message = !ok ? "The restore didn't finish. Open Details to see why." : dryRun ? "Preview done. Open Details to see what would be copied." : `Restored into ${target}.`;
		failed = !ok;
		if (ok && !dryRun) restoring = false;
	}
	async function saveWithSchedule(){await save();if(schedule!==originalSchedule){await invoke("set_backup_schedule",{schedule});originalSchedule=schedule;}}
	useDraft(() => ({ repo, password,schedule }), saveWithSchedule, () => st !== null && scheduleReady);
</script>

<ViewScaffold title="Backup">
	<Group title="Back up your files">
		<div class="warnbox"><Info size={16} /><span>Snapshots protect the system, not your home folder. Backups keep copies of your files somewhere else.</span></div>
		{#if !st}
			<p class="muted">Loading…</p>
		{:else if !st.restic_installed}
			<div class="ui-item">
				<div class="lb"><b>Set up file backups</b><small>Installs the backup tool (restic). Needs your password.</small></div>
				<button class="btn small primary" disabled={busy} onclick={installRestic}>{busy ? "Setting up…" : "Set up"}</button>
			</div>
		{:else}
			<Row label="Back up to">
				<div class="ui-seg">
					<button class:on={kind === "folder"} onclick={() => { if (kind !== "folder") repo = ""; }}>A folder or drive</button>
					<button class:on={kind === "ssh"} onclick={() => { if (kind !== "ssh") repo = "sftp:"; }}>Another computer</button>
				</div>
			</Row>
			{#if kind === "ssh"}
				<Row label="Computer and folder" hint="Over SSH, like you@server:/backups/laptop">
					<input class="ui-input" value={repo.replace(/^sftp:/, "")} placeholder="you@server:/backups/laptop" onchange={(e) => (repo = `sftp:${e.currentTarget.value.trim()}`)} />
				</Row>
			{:else}
				<FileField label="Folder" bind:value={repo} mode="folder" placeholder="Pick a folder, for example on a USB drive" />
			{/if}
			<PasswordField label={st.has_password ? "Backup password (saved)" : "Backup password"} bind:value={password} />
			<p class="muted">Backups are encrypted with this password. Keep it somewhere safe: without it they can't be read.</p>
			<Row label="How often" hint="Missed backups run when you next log in.">
				<div class="ui-seg">
					{#each [["off", "Only when I ask"], ["daily", "Every day"], ["weekly", "Every week"]] as [v, l] (v)}<button class:on={schedule === v} onclick={() => (schedule = v)}>{l}</button>{/each}
				</div>
			</Row>
			<div class="foot">
				<button class="btn primary small" disabled={busy || !repo.replace(/^sftp:/, "").trim()} onclick={backupNow}>{busy ? "Working…" : "Back up now"}</button>
				<button class="btn small" disabled={busy || !repo.trim()} onclick={openRestore}>Restore files…</button>
			</div>
			{#if message}<p class="muted" class:bad={failed} role="status">{message}</p>{/if}
		{/if}
		{#if log.length}<details class="ui-disclosure"><summary>Details</summary><LogView lines={log} /></details>{/if}
	</Group>
</ViewScaffold>

{#if restoring}
	<Sheet title="Restore files" lede="Files are copied into a new folder, so nothing in your home folder is overwritten." onClose={() => (restoring = false)}>
		{#if snapshots.length === 0}
			<p class="muted">{message || "Looking for backups…"}</p>
		{:else}
			<div class="snaps">
				{#each snapshots as snap (snap.id)}
					<button class="ui-item pick" class:on={selected === snap.id} onclick={() => (selected = snap.id)}>
						<div class="lb"><b>{new Date(snap.time).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })}</b><small>{snap.paths.join(", ")}</small></div>
					</button>
				{/each}
			</div>
			<FileField label="Put them in" bind:value={restoreTarget} mode="folder" />
		{/if}
		{#snippet actions()}
			<button class="btn" disabled={busy || snapshots.length === 0} onclick={() => restore(true)}>Preview</button>
			<button class="btn primary" disabled={busy || snapshots.length === 0} onclick={() => restore(false)}>{busy ? "Restoring…" : "Restore"}</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; line-height:1.45; } .bad { color:var(--red); }
	.foot { display:flex; gap:8px; flex-wrap:wrap; padding:6px 0 10px; }
	.snaps { max-height:260px; overflow:auto; margin-bottom:8px; }
	.pick { width:100%; background:none; border:0; border-radius:10px; text-align:left; color:inherit; cursor:pointer; padding:9px 8px; }
	.pick.on { background:var(--accent-soft); }
</style>
