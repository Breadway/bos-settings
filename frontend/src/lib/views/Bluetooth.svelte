<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Switch from "$lib/components/Switch.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import { changes } from "$lib/changes.svelte";
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
	import Plus from "@lucide/svelte/icons/plus";

	interface BtDevice {
		address: string;
		name: string;
		connected: boolean;
		battery: number | null;
		icon?: string;
		named?: boolean;
	}

	// BlueZ "Icon" values (freedesktop device icon names) -> a type icon and word.
	function kind(icon: string | undefined): { Icon: typeof BluetoothIcon; word: string } {
		const i = icon ?? "";
		if (i.startsWith("audio-head")) return { Icon: Headphones, word: "Headphones" };
		if (i === "input-gaming") return { Icon: Gamepad, word: "Controller" };
		if (i === "input-mouse") return { Icon: Mouse, word: "Mouse" };
		if (i === "input-tablet") return { Icon: Mouse, word: "Tablet" };
		if (i === "input-keyboard") return { Icon: Keyboard, word: "Keyboard" };
		if (i.startsWith("phone")) return { Icon: Smartphone, word: "Phone" };
		if (i.startsWith("audio-")) return { Icon: Speaker, word: "Speaker" };
		if (i === "computer") return { Icon: Laptop, word: "Computer" };
		if (i.includes("watch")) return { Icon: Watch, word: "Watch" };
		return { Icon: BluetoothIcon, word: "" };
	}

	let powered = $state<boolean | null | undefined>(undefined);
	let paired = $state<BtDevice[] | null>(null);
	let busy = $state<string | null>(null);
	let error = $state("");
	let adding = $state(false);
	let found = $state<BtDevice[] | null>(null);
	let scanning = $state(false);
	let pairError = $state("");

	// Two identical controllers read the same; the last address digits tell them apart.
	let dupNames = $derived(new Set((paired ?? []).map((d) => d.name).filter((n, i, all) => all.indexOf(n) !== i)));
	let named = $derived(found?.filter((d) => d.named !== false) ?? []);
	let unnamed = $derived(found?.filter((d) => d.named === false) ?? []);

	async function refreshPaired() {
		paired = await invoke<BtDevice[]>("get_paired_devices");
	}

	onMount(async () => {
		powered = await invoke<boolean | null>("get_adapter_powered");
		if (powered !== null) await refreshPaired();
	});

	async function togglePower(on: boolean) {
		error = "";
		try { await invoke("set_adapter_powered", { on }); powered = on; if (on) await refreshPaired(); }
		catch (e) { error = `Couldn't turn Bluetooth ${on ? "on" : "off"}: ${e}`; }
	}

	async function toggleConnect(dev: BtDevice) {
		error = "";
		busy = dev.address;
		try {
			await invoke(dev.connected ? "bt_disconnect" : "bt_connect", { address: dev.address });
		} catch (e) {
			error = dev.connected ? `Couldn't disconnect ${dev.name}: ${e}` : `Couldn't connect ${dev.name}. Make sure it is on and nearby.`;
		}
		busy = null;
		await refreshPaired();
	}

	async function forget(dev: BtDevice) {
		if (!(await confirmAction({ title: `Forget ${dev.name}?`, message: "You'll need to pair it again to use it.", confirmLabel: "Forget" }))) return;
		error = "";
		try { await invoke("bt_forget", { address: dev.address }); changes.message = `Forgot ${dev.name}`; changes.undo = null; }
		catch (e) { error = `Couldn't forget ${dev.name}: ${e}`; }
		await refreshPaired();
	}

	async function scan() {
		scanning = true;
		pairError = "";
		try { found = await invoke<BtDevice[]>("scan_bluetooth"); }
		catch (e) { pairError = String(e); }
		finally { scanning = false; }
	}

	function openAdd() {
		adding = true;
		found = null;
		void scan();
	}

	async function pair(dev: BtDevice) {
		pairError = "";
		busy = dev.address;
		try {
			await invoke("bt_pair", { address: dev.address });
			await invoke("bt_connect", { address: dev.address }).catch(() => {});
			await refreshPaired();
			adding = false;
			changes.message = `${dev.named === false ? "Device" : dev.name} paired`;
			changes.undo = null;
		} catch {
			pairError = `Couldn't pair ${dev.named === false ? "that device" : dev.name}. Put it in pairing mode and try again. Devices that ask for a PIN can't be paired here yet.`;
		}
		busy = null;
	}

	function shortAddr(a: string) {
		return `…${a.slice(-5)}`;
	}
</script>

<ViewScaffold title="Bluetooth" lede="Your devices, with battery where they report it.">
	{#if powered === undefined}
		<p class="muted">Loading…</p>
	{:else if powered === null}
		<EmptyState icon={BluetoothOff} title="No Bluetooth adapter found" hint="This computer doesn't have Bluetooth, or it is switched off in the firmware settings." />
	{:else}
		<div class="hero-card">
			<span class="big">{#if powered}<BluetoothIcon size={26} />{:else}<BluetoothOff size={26} />{/if}</span>
			<div>
				<h3>Bluetooth is {powered ? "on" : "off"}</h3>
				<p>{powered ? `${paired?.filter((d) => d.connected).length ?? 0} connected · ${paired?.length ?? 0} paired` : "Turn it on to use your headphones, controllers and other devices."}</p>
			</div>
			<div class="acts">
				{#if powered}<button class="btn primary add" onclick={openAdd}><Plus size={15} />Add device</button>{/if}
				<Switch bind:value={() => powered ?? false, (v) => togglePower(v)} ariaLabel="Bluetooth" />
			</div>
		</div>
		{#if error}<p class="err" role="alert">{error}</p>{/if}

		{#if powered}
			{#if paired === null}
				<p class="muted">Loading devices…</p>
			{:else if paired.length === 0}
				<EmptyState icon={BluetoothIcon} title="No devices yet" hint="Press Add device and put your device in pairing mode." />
			{:else}
				<div class="devgrid">
					{#each paired as dev (dev.address)}
						{@const k = kind(dev.icon)}
						<div class="dev-card" class:on={dev.connected} data-setting={dev.name}>
							<div class="t">
								<span class="ico"><k.Icon size={18} /></span>
								<div>
									<b>{dev.name}</b>
									<small>{dev.connected ? "Connected" : "Not connected"}{dev.battery == null ? "" : ` · ${dev.battery}% battery`}{dupNames.has(dev.name) ? ` · ${shortAddr(dev.address)}` : ""}</small>
								</div>
							</div>
							<div class="acts">
								<button class="btn small" class:primary={!dev.connected} disabled={busy === dev.address} onclick={() => toggleConnect(dev)}>
									{busy === dev.address ? (dev.connected ? "Disconnecting…" : "Connecting…") : dev.connected ? "Disconnect" : "Connect"}
								</button>
								<button class="btn small ghost" onclick={() => forget(dev)}>Forget</button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	{/if}
</ViewScaffold>

{#if adding}
	<Sheet title="Add a device" lede="Put your device in pairing mode. It shows up here." onClose={() => (adding = false)}>
		{#if found === null}
			<p class="muted">Looking for devices…</p>
		{:else if named.length === 0}
			<p class="muted">Nothing found yet. Check the device is in pairing mode, then scan again.</p>
		{/if}
		{#each named as dev (dev.address)}
			{@const k = kind(dev.icon)}
			<div class="ui-item">
				<span class="ico"><k.Icon size={18} /></span>
				<div class="lb"><b>{dev.name}</b><small>{k.word || "Device"}</small></div>
				<button class="btn small primary" disabled={busy !== null} onclick={() => pair(dev)}>{busy === dev.address ? "Pairing…" : "Pair"}</button>
			</div>
		{/each}
		{#if unnamed.length}
			<details class="ui-disclosure">
				<summary>{unnamed.length} nearby {unnamed.length === 1 ? "device doesn't" : "devices don't"} share a name</summary>
				<p class="muted small">Usually phones, trackers and beacons. Pair one only if you know which it is.</p>
				{#each unnamed as dev (dev.address)}
					<div class="ui-item">
						<div class="lb"><b>Unnamed device</b><small>{dev.address}</small></div>
						<button class="btn small" disabled={busy !== null} onclick={() => pair(dev)}>{busy === dev.address ? "Pairing…" : "Pair"}</button>
					</div>
				{/each}
			</details>
		{/if}
		{#if pairError}<p class="err" role="alert">{pairError}</p>{/if}
		{#snippet actions()}
			<button class="btn" disabled={scanning} onclick={scan}>{scanning ? "Scanning…" : "Scan again"}</button>
			<button class="btn primary" onclick={() => (adding = false)}>Done</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px; }
	.muted.small { font-size:12px; margin:0 2px 6px; }
	.err { color:var(--red); font-size:12.5px; margin:0 2px; }
	.add { display:inline-flex; align-items:center; gap:6px; }
</style>
