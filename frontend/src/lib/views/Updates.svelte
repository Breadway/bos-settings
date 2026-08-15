<script lang="ts">
	import { getContext, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import { NAVIGATE_KEY, type Navigate } from "$lib/nav";
	import Download from "@lucide/svelte/icons/download";
	import Cpu from "@lucide/svelte/icons/cpu";

	const navigate = getContext<Navigate | undefined>(NAVIGATE_KEY);

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
	let log = $state<string[]>([]);
	let busy = $state(false);

	async function refresh() {
		status = await invoke<UpdatesStatus>("get_updates_status");
	}

	onMount(refresh);

	function appendLine(line: string) {
		log = [...log, line];
	}

	async function run(command: string, args: Record<string, unknown> = {}) {
		log = [];
		busy = true;
		await runStreamed(command, args, appendLine);
		busy = false;
		await refresh();
	}
</script>

<ViewScaffold title="Updates">
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
					disabled={busy}
					onclick={() => run("nvidia_setup")}
				>
					{status.nvidia.installed ? "Re-run setup" : "Install driver"}
				</button>
			</div>
		</Group>
	{/if}

	<Group
		title="System packages (pacman)"
		hint="The same set Packages covers with pacman -Syu. Needs your password (polkit)."
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
			<button disabled={busy} class="primary" onclick={() => run("pacman_system_update")}>Update system</button>
			<button disabled={busy} onclick={refresh}>Refresh</button>
		</div>
	</Group>

	<Group
		title="Bread ecosystem (bakery)"
		hint="bakery --dry-run update --all — bakery has no separate outdated command. Per-package install still lives on Packages."
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
						<button disabled={busy} onclick={() => run("bakery_update", { name: pkg.name })}>Update</button>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button disabled={busy} class="primary" onclick={() => run("bakery_update_all")}>Update all bakery</button>
		</div>
	</Group>

	<Group title="Firmware (fwupd)" hint="Same list as the Firmware page. fwupd-refresh.timer already refreshes metadata in the background." wide>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.firmware.length === 0}
			<EmptyState icon={Download} title="No updatable firmware" hint="Not every device speaks fwupd." />
		{:else}
			<div class="list">
				{#each status.firmware as dev (dev.name)}
					<div class="row">
						<span class="name">{dev.name}</span>
						<span class="version">{dev.version}</span>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button disabled={busy} onclick={() => run("fwupd_refresh")}>Check for updates</button>
			<button disabled={busy} class="primary" onclick={() => run("fwupd_update")}>Update firmware</button>
		</div>
	</Group>

	<Group
		title="Rollback"
		hint="Boot a snapshot from GRUB (BOS snapshots)."
	>
		<Hint text="Snapshots lists number, date, and description so you know which GRUB entry to pick. snapper rollback will not change what GRUB boots (rootflags=subvol=@)." />
		<Hint text="Bakery also has bakery rollback <pkg> for a single ecosystem binary — that is not a system rollback." />
		<div class="btn-row">
			<button disabled={!navigate} onclick={() => navigate?.("snapshots")}>Open Snapshots</button>
		</div>
	</Group>

	<LogView lines={log} />
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
</style>
