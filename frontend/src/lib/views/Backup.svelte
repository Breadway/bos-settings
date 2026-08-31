<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import PasswordField from "$lib/components/PasswordField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import Archive from "@lucide/svelte/icons/archive";

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

	async function run(command: string, args: Record<string, unknown> = {}) {
		log = [];
		busy = true;
		message = "";
		const ok = await runStreamed(command, args, (line) => {
			log = [...log, line];
		});
		busy = false;
		if (!ok) message = "Failed. See the log.";
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
			message = "Refusing to restore onto $HOME. Leave the default ~/bos-restore-<id> or pick another folder.";
			return;
		}
		if (!dryRun) {
			const ok = confirm(
				`Restore snapshot ${snap} into ${target}?\n\nFiles go into that directory. Your live home is not overwritten.`,
			);
			if (!ok) return;
		}
		await run(dryRun ? "restic_restore_dry_run" : "restic_restore", {
			snapshot: snap,
			target,
		});
	}
</script>

<ViewScaffold title="Backup">
	<Group
		title="Repository"
		hint="Local folder or sftp. Password is write-only."
		wide
	>
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.restic_installed}
			<Hint text="restic is not installed." />
			<button class="primary" disabled={busy} onclick={installRestic}>Install restic</button>
		{:else}
			<FileField label="Local path" bind:value={repo} mode="folder" placeholder="/mnt/backup/bos" />
			<TextField label="Or SFTP" bind:value={repo} placeholder="sftp:user@host:/backups/bos" />
			<PasswordField label={st.has_password ? "Password (leave empty to keep)" : "Password"} bind:value={password} />
			<SaveButton onSave={save} />
		{/if}
	</Group>

	<Group
		title="Actions"
		hint="Home directory. Snapshots page is the root filesystem."
	>
		<div class="btn-row">
			<button disabled={busy} onclick={() => run("restic_init")}>Init repo</button>
			<button class="primary" disabled={busy} onclick={() => run("restic_backup")}>Backup home</button>
			<button disabled={busy} onclick={listSnaps}>List snapshots</button>
		</div>
		{#if message}<Hint text={message} />{/if}
	</Group>

	<Group
		title="Restore"
		hint="Writes into a new folder (default ~/bos-restore-<id>). Does not overwrite $HOME. Dry-run previews the same target."
	>
		<FileField label="Restore into" bind:value={restoreTarget} mode="folder" placeholder="/home/you/bos-restore-latest" />
		<div class="btn-row">
			<button disabled={busy || !st?.restic_installed} onclick={() => restore(true)}>Restore dry-run</button>
			<button class="primary" disabled={busy || !st?.restic_installed} onclick={() => restore(false)}>Restore</button>
		</div>
	</Group>

	<Group title="Snapshots" wide>
		{#if snapshots.length === 0}
			<EmptyState icon={Archive} title="No snapshots loaded" hint="Init, backup, then list." />
		{:else}
			<div class="list">
				{#each snapshots as s (s.id)}
					<button class="row" class:selected={selected === s.id} onclick={() => (selected = s.id)}>
						<span class="id">{s.id}</span>
						<span class="time">{s.time}</span>
						<span class="paths">{s.paths.join(", ")}</span>
					</button>
				{/each}
			</div>
		{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>

<style>
	.btn-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm, 8px);
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		max-height: 240px;
		overflow-y: auto;
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
	}

	.row.selected {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	.id {
		width: 10ch;
		flex-shrink: 0;
		font-family: monospace;
	}

	.time {
		width: 22ch;
		flex-shrink: 0;
		opacity: 0.85;
	}

	.paths {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
</style>
