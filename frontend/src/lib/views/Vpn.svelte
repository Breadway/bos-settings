<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Switch from "$lib/components/Switch.svelte";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Plus from "@lucide/svelte/icons/plus";

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
	let message = $state("");
	let busy = $state(false);

	const KIND: Record<string, string> = { wireguard: "WireGuard", vpn: "VPN", openvpn: "OpenVPN" };

	async function refresh() {
		status = await invoke<VpnStatus>("get_vpn_connections");
	}

	onMount(refresh);

	async function connect(name: string, up: boolean) {
		busy = true;
		message = up ? `Connecting ${name}…` : "";
		try {
			await invoke(up ? "vpn_connect" : "vpn_disconnect", { name });
			message = "";
			await refresh();
		} catch (e) {
			message = `Couldn't ${up ? "connect" : "disconnect"} ${name}: ${e}`;
		}
		busy = false;
	}

	async function addVpn() {
		const picked = await open({ filters: [{ name: "WireGuard or OpenVPN file", extensions: ["conf", "ovpn"] }] });
		if (typeof picked !== "string") return;
		busy = true;
		message = "Adding…";
		try {
			await invoke("vpn_import", { path: picked });
			message = "Added. Turn it on below.";
			await refresh();
		} catch (e) {
			message = `Couldn't add that file: ${e}`;
		}
		busy = false;
	}
</script>

<ViewScaffold title="Other connections">
	<Group title="VPN">
		{#if !status}
			<p class="empty">Loading…</p>
		{:else if status.error}
			<p class="empty">Couldn't list VPN connections: {status.error}</p>
		{:else if status.connections.length === 0}
			<p class="empty">No VPN yet. Add a WireGuard (.conf) or OpenVPN (.ovpn) file from your provider. Tailscale is set per place, above.</p>
		{:else}
			{#each status.connections as c (c.name)}
				<div class="ui-item" data-setting={c.name}>
					<span class="ico"><ShieldCheck size={18} /></span>
					<div class="lb"><b>{c.name}</b><small>{KIND[c.kind] ?? c.kind}{c.active ? " · connected" : ""}{c.autoconnect ? " · connects by itself" : ""}</small></div>
					<Switch disabled={busy} bind:value={() => c.active, (v) => connect(c.name, v)} ariaLabel={c.name} />
				</div>
			{/each}
		{/if}
		{#if message}<p class="empty" role="status">{message}</p>{/if}
		<div class="foot"><button class="btn small" disabled={busy} onclick={addVpn}><Plus size={14} />Add VPN</button></div>
	</Group>
</ViewScaffold>

<style>
	.empty { color:var(--muted); font-size:12.5px; margin:6px 2px 10px; line-height:1.45; }
	.foot { padding:4px 0 10px; }
	.foot .btn { display:inline-flex; align-items:center; gap:6px; }
</style>
