<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Wifi from "@lucide/svelte/icons/wifi";
	import WifiOff from "@lucide/svelte/icons/wifi-off";
	import Lock from "@lucide/svelte/icons/lock";

	interface WifiNetwork {
		ssid: string;
		signal: number;
		secured: boolean;
		active: boolean;
		known: boolean;
	}

	let radioEnabled = $state(false);
	let ethernet = $state<string | null>(null);
	let networks = $state<WifiNetwork[] | null>(null);
	let scanning = $state(false);
	let status = $state("");
	let pendingSsid = $state<string | null>(null);
	let password = $state("");

	const VIRTUAL_IFACE_PREFIXES = ["veth", "docker", "br-", "virbr", "vnet", "tun", "tap", "vmnet", "podman"];
	const ETHERNET_STATE_LABELS: Record<string, string> = {
		connected: "Connected",
		connecting: "Connecting…",
		disconnected: "Not connected",
		disconnecting: "Disconnecting…",
		unavailable: "Not connected",
		unmanaged: "Not connected",
	};
	let ethernetLabel = $derived.by(() => {
		if (!ethernet) return null;
		const [iface, rawState] = ethernet.split(": ");
		if (!iface || VIRTUAL_IFACE_PREFIXES.some((prefix) => iface.startsWith(prefix))) return null;
		return ETHERNET_STATE_LABELS[rawState ?? ""] ?? "Not connected";
	});
	let activeNet = $derived(networks?.find((n) => n.active) ?? null);
	let nearby = $derived(networks?.filter((n) => !n.active) ?? []);

	onMount(async () => {
		const info = await invoke<{ radio_enabled: boolean; ethernet: string | null }>("get_network_info");
		radioEnabled = info.radio_enabled;
		ethernet = info.ethernet;
		if (info.radio_enabled) await scan();
	});

	async function toggleRadio(enabled: boolean) {
		radioEnabled = enabled;
		await invoke("set_wifi_radio", { enabled });
	}

	async function scan() {
		scanning = true;
		try {
			networks = await invoke<WifiNetwork[]>("scan_wifi");
		} finally {
			scanning = false;
			status = "";
		}
	}

	async function connect(ssid: string, secured: boolean, known: boolean) {
		if (secured && !known) {
			pendingSsid = ssid;
			return;
		}
		status = `Connecting to ${ssid}…`;
		try {
			await invoke("connect_wifi", { ssid, password: null });
			status = `Connected to ${ssid}`;
			await scan();
		} catch (e) {
			status = `Failed: ${e}`;
		}
	}

	async function connectWithPassword() {
		const ssid = pendingSsid!;
		const pw = password;
		pendingSsid = null;
		password = "";
		status = `Connecting to ${ssid}…`;
		try {
			await invoke("connect_wifi", { ssid, password: pw });
			status = `Connected to ${ssid}`;
			await scan();
		} catch {
			status = "Wrong password?";
		}
	}

	function bars(signal: number): number {
		if (signal >= 75) return 4;
		if (signal >= 50) return 3;
		if (signal >= 25) return 2;
		return 1;
	}
</script>

<ViewScaffold title="Wi-Fi">
	<Group title="Radio">
		<SwitchField label="Wi-Fi" hint={radioEnabled ? "On" : "Off"} bind:value={() => radioEnabled, (v) => toggleRadio(v)} />
		{#if ethernetLabel}
			<InfoRow label="Ethernet" value={ethernetLabel} />
		{/if}
	</Group>

	{#if activeNet}
		<Group title="This network">
			<div class="wifi">
				<div class="bars on">
					{#each [1, 2, 3, 4] as n (n)}
						<b style="height: {3 + n * 3}px" class:lit={bars(activeNet.signal) >= n}></b>
					{/each}
				</div>
				<div class="ssid">
					<strong>{activeNet.ssid}</strong>
					<small>{activeNet.secured ? "Secured" : "Open"} · {activeNet.signal}%</small>
				</div>
				<span class="pill on">Connected</span>
			</div>
		</Group>
	{/if}

	<Group title="Nearby" wide>
		<div class="list">
			{#if scanning && networks === null}
				<div class="empty">
					<Wifi size={36} />
					<span>Scanning…</span>
				</div>
			{:else if networks === null}
				<div class="empty">
					<Wifi size={36} />
					<span>No scan yet</span>
				</div>
			{:else if nearby.length === 0 && !activeNet}
				<div class="empty">
					<WifiOff size={36} />
					<span>No networks found</span>
				</div>
			{:else}
				{#each nearby as net (net.ssid)}
					<div class="wifi">
						<div class="bars">
							{#each [1, 2, 3, 4] as n (n)}
								<b style="height: {3 + n * 3}px" class:lit={bars(net.signal) >= n}></b>
							{/each}
						</div>
						<div class="ssid">
							{net.ssid}
							<small>{net.known ? "Saved" : net.secured ? "Secured" : "Open"}</small>
						</div>
						{#if net.secured}<Lock size={14} />{/if}
						<button class="btn" onclick={() => connect(net.ssid, net.secured, net.known)}>Connect</button>
					</div>
				{/each}
			{/if}
		</div>

		{#if pendingSsid}
			<div class="pw-row">
				<input type="password" bind:value={password} placeholder="Password for {pendingSsid}" />
				<button class="btn primary" onclick={connectWithPassword}>Connect</button>
			</div>
		{/if}
		{#if status}
			<span class="status">{status}</span>
		{/if}
		<button class="btn" disabled={scanning} onclick={scan}>{scanning ? "Scanning…" : "Scan"}</button>
	</Group>

	<Group title="Advanced">
		<button class="btn" onclick={() => invoke("open_connection_editor")}>Connection editor</button>
	</Group>
</ViewScaffold>

<style>
	.list {
		min-height: 72px;
		max-height: 280px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 20px 0;
		opacity: 0.6;
	}

	.wifi {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 2px;
	}

	.wifi + .wifi {
		border-top: 1px solid var(--line);
	}

	.bars {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 14px;
		width: 16px;
		flex-shrink: 0;
	}

	.bars b {
		width: 3px;
		background: color-mix(in srgb, var(--fg) 22%, transparent);
		border-radius: 1px;
		display: block;
	}

	.bars b.lit,
	.bars.on b.lit {
		background: var(--accent);
	}

	.ssid {
		flex: 1;
		min-width: 0;
	}

	.ssid small {
		display: block;
		color: var(--muted);
		font-size: 11px;
	}

	.pw-row {
		display: flex;
		gap: 8px;
		margin-top: 8px;
	}

	.pw-row input {
		flex: 1;
		background: var(--bg);
		border: 1px solid transparent;
		border-radius: 10px;
		padding: 6px 10px;
	}

	.status {
		display: block;
		margin: 8px 0;
		color: var(--muted);
		font-size: 12px;
	}

	.btn {
		margin-top: 10px;
		align-self: flex-start;
	}

	.wifi .btn {
		margin-top: 0;
	}
</style>
