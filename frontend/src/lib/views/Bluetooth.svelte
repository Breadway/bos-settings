<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import BluetoothIcon from "@lucide/svelte/icons/bluetooth";
	import BluetoothOff from "@lucide/svelte/icons/bluetooth-off";

	interface BtDevice {
		address: string;
		name: string;
		connected: boolean;
	}

	let powered = $state<boolean | null>(null);
	let paired = $state<BtDevice[] | null>(null);
	let scanResults = $state<BtDevice[] | null>(null);
	let scanning = $state(false);
	let log = $state<string[]>([]);

	async function refreshPaired() {
		paired = await invoke<BtDevice[]>("get_paired_devices");
	}

	onMount(async () => {
		powered = await invoke<boolean | null>("get_adapter_powered");
		if (powered !== null) await refreshPaired();
	});

	async function togglePower(on: boolean) {
		powered = on;
		await invoke("set_adapter_powered", { on });
	}

	async function toggleConnect(dev: BtDevice) {
		log = [];
		try {
			await invoke(dev.connected ? "bt_disconnect" : "bt_connect", { address: dev.address });
		} catch (e) {
			log = [...log, `${e}`];
		}
		await refreshPaired();
	}

	async function forget(dev: BtDevice) {
		if (!confirm(`Forget "${dev.name}"? You'll need to pair it again to reconnect.`)) return;
		log = [];
		try {
			await invoke("bt_forget", { address: dev.address });
		} catch (e) {
			log = [...log, `${e}`];
		}
		await refreshPaired();
	}

	async function scan() {
		scanning = true;
		scanResults = await invoke<BtDevice[]>("scan_bluetooth");
		scanning = false;
	}

	async function pair(dev: BtDevice) {
		log = [];
		try {
			await invoke("bt_pair", { address: dev.address });
			await refreshPaired();
			scanResults = scanResults!.filter((d) => d.address !== dev.address);
		} catch (e) {
			log = [...log, `${e}`];
		}
	}
</script>

<ViewScaffold title="Bluetooth">
	{#if powered === null}
		<EmptyState
			icon={BluetoothOff}
			title="No Bluetooth adapter found"
			hint="This machine doesn't have Bluetooth hardware, or the kernel module isn't loaded."
		/>
	{:else}
		<Group title="Adapter">
			<SwitchField label="Bluetooth" bind:value={() => powered ?? false, (v) => togglePower(v)} />
		</Group>

		<Group title="Paired devices">
			<div class="list">
				{#if paired === null}
					<Hint text="Loading…" />
				{:else if paired.length === 0}
					<EmptyState icon={BluetoothIcon} title="No paired devices" hint="Scan and pair a device to see it here." />
				{:else}
					{#each paired as dev (dev.address)}
						<div class="row">
							<span class="name" class:active={dev.connected}>{dev.name}{dev.connected ? " (connected)" : ""}</span>
							<button class="action" onclick={() => toggleConnect(dev)}>{dev.connected ? "Disconnect" : "Connect"}</button>
							<button class="remove" onclick={() => forget(dev)}>Forget</button>
						</div>
					{/each}
				{/if}
			</div>
		</Group>

		<Group
			title="Available devices"
			hint="Scanning takes a few seconds. Devices needing a PIN aren't supported — only &quot;just works&quot; pairing (most headphones, speakers, keyboards, and mice)."
		>
			<div class="list">
				{#if scanResults === null}
					<EmptyState icon={BluetoothIcon} title="Not scanned yet" hint="Press Scan to look for nearby devices." />
				{:else if scanResults.length === 0}
					<EmptyState icon={BluetoothIcon} title="No new devices found" hint="Make sure the device is powered on and in pairing mode, then Scan again." />
				{:else}
					{#each scanResults as dev (dev.address)}
						<div class="row">
							<span class="name">{dev.name}</span>
							<button class="action" onclick={() => pair(dev)}>Pair</button>
						</div>
					{/each}
				{/if}
			</div>
			<button class="scan" disabled={scanning} onclick={scan}>{scanning ? "Scanning…" : "Scan"}</button>
		</Group>

		<LogView lines={log} />
	{/if}
</ViewScaffold>

<style>
	.list {
		min-height: 60px;
		max-height: 220px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		background-color: var(--surface);
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.name {
		flex: 1;
	}

	.name.active {
		font-weight: bold;
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.action {
		background-color: var(--surface);
		color: var(--on-surface);
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
	}

	.scan {
		background-color: var(--accent);
		color: var(--on-accent);
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
	}

	.scan:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
