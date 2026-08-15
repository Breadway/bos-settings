<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import ShieldEllipsis from "@lucide/svelte/icons/shield-ellipsis";

	interface VpnConnection {
		name: string;
		kind: string;
		active: boolean;
		autoconnect: boolean;
	}
	interface VpnStatus {
		connections: VpnConnection[];
		error: string | null;
	}

	let status = $state<VpnStatus | null>(null);
	let importPath = $state("");
	let message = $state("");
	let busy = $state(false);

	async function refresh() {
		status = await invoke<VpnStatus>("get_vpn_connections");
	}

	onMount(refresh);

	async function connect(name: string, up: boolean) {
		busy = true;
		message = "";
		try {
			await invoke(up ? "vpn_connect" : "vpn_disconnect", { name });
			message = up ? `Connected ${name}` : `Disconnected ${name}`;
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
		busy = false;
	}

	async function doImport() {
		if (!importPath.trim()) return;
		busy = true;
		message = "";
		try {
			await invoke("vpn_import", { path: importPath.trim() });
			message = "Imported. Connect it from the list.";
			importPath = "";
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
		busy = false;
	}
</script>

<ViewScaffold title="VPN / WireGuard">
	<Group
		title="NetworkManager tunnels"
		hint="Lists connection type vpn and wireguard only. Wi-Fi profiles stay on breadcrumbs; Tailscale is not managed here."
		wide
	>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.error}
			<EmptyState icon={ShieldEllipsis} title="Couldn't list connections" hint={status.error} />
		{:else if status.connections.length === 0}
			<EmptyState icon={ShieldEllipsis} title="No VPN or WireGuard connections" hint="Import a .conf below, or use nm-connection-editor from Network." />
		{:else}
			<div class="list">
				{#each status.connections as c (c.name)}
					<div class="row">
						<div class="meta">
							<span class="name">{c.name}</span>
							<span class="kind">{c.kind}{c.active ? " · connected" : ""}{c.autoconnect ? " · autoconnect" : ""}</span>
						</div>
						{#if c.active}
							<button disabled={busy} onclick={() => connect(c.name, false)}>Disconnect</button>
						{:else}
							<button class="primary" disabled={busy} onclick={() => connect(c.name, true)}>Connect</button>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
		<button onclick={refresh}>Refresh</button>
		{#if message}<Hint text={message} />{/if}
	</Group>

	<Group title="Import" hint="WireGuard .conf or OpenVPN .ovpn. NetworkManager stores the secret after import.">
		<FileField label="Config" bind:value={importPath} placeholder="wg0.conf" extensions={["conf", "ovpn"]} />
		<button class="primary" disabled={busy || !importPath.trim()} onclick={doImport}>Import</button>
	</Group>
</ViewScaffold>

<style>
	.list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-bottom: var(--space-sm, 8px);
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.meta {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.kind {
		opacity: 0.7;
		font-size: var(--font-size-secondary, 12px);
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
