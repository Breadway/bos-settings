<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Hint from "$lib/components/Hint.svelte";
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

	// The backend reports the first `nmcli`-visible device of TYPE=ethernet,
	// which on a machine running containers/VMs can be a virtual interface
	// (docker/podman veth pairs, libvirt bridges, VPN tuns) rather than a
	// real NIC — nmcli doesn't distinguish these from physical ethernet.
	// Names like "vethYCF3ZA: unmanaged" are meaningless to a non-technical
	// user, so hide the card rather than show raw interface jargon.
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

	onMount(async () => {
		const info = await invoke<{ radio_enabled: boolean; ethernet: string | null }>("get_network_info");
		radioEnabled = info.radio_enabled;
		ethernet = info.ethernet;
	});

	async function toggleRadio(enabled: boolean) {
		radioEnabled = enabled;
		await invoke("set_wifi_radio", { enabled });
	}

	async function scan() {
		scanning = true;
		status = "Scanning…";
		networks = await invoke<WifiNetwork[]>("scan_wifi");
		status = `Found ${networks.length} network(s)`;
		scanning = false;
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
		} catch (e) {
			status = `Failed to connect to ${ssid}: ${e}`;
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
		} catch {
			status = `Failed to connect to ${ssid}: wrong password?`;
		}
	}

	function signalLabel(signal: number): string {
		return `${Math.min(100, Math.max(0, signal))}%`;
	}
</script>

<ViewScaffold title="Network">
	<Group title="Wi-Fi">
		<Row label="Wi-Fi radio">
			<button class="switch" class:on={radioEnabled} role="switch" aria-checked={radioEnabled} aria-label="Wi-Fi radio" onclick={() => toggleRadio(!radioEnabled)}>
				<span class="knob"></span>
			</button>
		</Row>
	</Group>

	{#if ethernetLabel}
		<Group title="Ethernet">
			<InfoRow label="Status" value={ethernetLabel} />
		</Group>
	{/if}

	<Group title="Available networks" wide>
		<div class="list">
			{#if networks === null}
				<div class="empty">
					<Wifi size={40} />
					<span>Not scanned yet</span>
					<span class="hint">Press Scan to see nearby networks.</span>
				</div>
			{:else if networks.length === 0}
				<div class="empty">
					<WifiOff size={40} />
					<span>No networks found</span>
					<span class="hint">Try Scan again, or check Wi-Fi radio is on.</span>
				</div>
			{:else}
				{#each networks as net (net.ssid)}
					<div class="net-row">
						<span class="ssid" class:active={net.active}>{net.ssid}{net.active ? " (connected)" : ""}</span>
						{#if net.secured}<Lock size={14} />{/if}
						<span class="signal">{signalLabel(net.signal)}</span>
						{#if !net.active}
							<button class="connect" onclick={() => connect(net.ssid, net.secured, net.known)}>Connect</button>
						{/if}
					</div>
				{/each}
			{/if}
		</div>

		{#if pendingSsid}
			<div class="pw-row">
				<input type="password" bind:value={password} placeholder="Password" />
				<button class="connect" onclick={connectWithPassword}>Connect</button>
			</div>
		{/if}
		{#if status}
			<span class="status">{status}</span>
		{/if}

		<button class="scan" disabled={scanning} onclick={scan}>{scanning ? "Scanning…" : "Scan"}</button>
	</Group>

	<Group title="Advanced" hint="VPN, 802.1x, and static IP configuration aren't covered here.">
		<button class="secondary" onclick={() => invoke("open_connection_editor")}>Open connection editor</button>
	</Group>
</ViewScaffold>

<style>
	.switch {
		width: 40px;
		height: 22px;
		border-radius: 999px;
		border: none;
		background-color: var(--overlay);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
	}
	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}
	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
	}

	.list {
		min-height: 100px;
		max-height: 260px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: var(--space-xl, 20px) 0;
		opacity: 0.6;
	}

	.hint {
		font-size: var(--font-size-secondary, 12px);
	}

	.net-row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		background-color: var(--surface);
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.ssid {
		flex: 1;
	}

	.ssid.active {
		font-weight: bold;
	}

	.signal {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.pw-row {
		display: flex;
		gap: var(--space-sm, 8px);
	}

	.pw-row input {
		flex: 1;
		background-color: var(--surface);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	button.connect,
	button.scan,
	button.secondary {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button.secondary {
		background-color: var(--bg);
		color: var(--on-surface);
		align-self: flex-start;
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
	}

	button.scan {
		align-self: flex-start;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
