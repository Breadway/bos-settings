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
	import Headphones from "@lucide/svelte/icons/headphones";
	import Gamepad from "@lucide/svelte/icons/gamepad-2";
	import Mouse from "@lucide/svelte/icons/mouse";
	import Keyboard from "@lucide/svelte/icons/keyboard";
	import Smartphone from "@lucide/svelte/icons/smartphone";
	import Speaker from "@lucide/svelte/icons/speaker";
	import Laptop from "@lucide/svelte/icons/laptop";
	import Watch from "@lucide/svelte/icons/watch";
	// BlueZ "Icon" values (freedesktop device icon names) -> a type icon for the row.
	function deviceIcon(icon: string | undefined) {
		const i = icon ?? "";
		if (i.startsWith("audio-head")) return Headphones;
		if (i === "input-gaming") return Gamepad;
		if (i === "input-mouse" || i === "input-tablet") return Mouse;
		if (i === "input-keyboard") return Keyboard;
		if (i.startsWith("phone")) return Smartphone;
		if (i.startsWith("audio-")) return Speaker;
		if (i === "computer") return Laptop;
		if (i.includes("watch")) return Watch;
		return BluetoothIcon;
	}

	interface BtDevice {
		address: string;
		name: string;
		connected: boolean;
        battery:number|null;
	icon?:string;
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
		try { await invoke("set_adapter_powered", { on }); powered = on; } catch (e) { log = [String(e)]; }
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
		try { scanResults = await invoke<BtDevice[]>("scan_bluetooth"); } catch (e) { log = [String(e)]; } finally { scanning = false; }
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

<ViewScaffold
	title="Bluetooth"
	lede="Paired devices and nearby scan."
>
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
						{@const Icon = deviceIcon(dev.icon)}
						<div class="row">
							<span class="type" aria-hidden="true"><Icon size={18} /></span>
							<div class="name" class:active={dev.connected}>{dev.name}<small>{dev.address} · {dev.connected ? "Connected" : "Not connected"}{dev.battery==null ? "" : ` · ${dev.battery}% battery`}</small></div>
							<button class="btn" onclick={() => toggleConnect(dev)}>{dev.connected ? "Disconnect" : "Connect"}</button>
							<button class="remove" onclick={() => forget(dev)}>Forget</button>
						</div>
					{/each}
				{/if}
			</div>
		</Group>

		<Group
			title="Available devices"
			hint="PIN pairing is not supported."
		>
			<div class="list">
				{#if scanResults === null}
					<EmptyState icon={BluetoothIcon} title="Not scanned yet" hint="Press Scan to look for nearby devices." />
				{:else if scanResults.length === 0}
					<EmptyState icon={BluetoothIcon} title="No new devices found" hint="Make sure the device is powered on and in pairing mode, then Scan again." />
				{:else}
					{#each scanResults as dev (dev.address)}
						<div class="row">
							<div class="name">{dev.name}<small>{dev.address}</small></div>
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
	.type { width: 34px; height: 34px; flex: none; border-radius: 10px; display: grid; place-items: center; background: var(--surface-2); color: var(--accent); }
	.list {
		min-height: 60px;

		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		padding: 10px 2px;
		border-top: 1px solid var(--line);
	}

	.row:first-child {
		border-top: none;
		padding-top: 0;
	}

	.name {
		min-width: 0;
		overflow-wrap: anywhere;
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
		background-color: transparent;
		color: var(--red);
		border: 1px solid color-mix(in srgb, var(--red) 40%, transparent);
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
	.name small { display:block; color:var(--muted); font-size:11px; font-weight:400; margin-top:3px; }
</style>
