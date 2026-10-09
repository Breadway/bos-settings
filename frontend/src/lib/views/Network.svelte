<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Switch from "$lib/components/Switch.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Places from "./Places.svelte";
	import { useDraft } from "$lib/drafts.svelte";
	import { recordChange } from "$lib/changes.svelte";
	import { crumbs, loadCrumbs, saveCrumbs, type SavedNetwork } from "$lib/breadcrumbs.svelte";
	import Wifi from "@lucide/svelte/icons/wifi";
	import WifiOff from "@lucide/svelte/icons/wifi-off";
	import Cable from "@lucide/svelte/icons/cable";
	import Lock from "@lucide/svelte/icons/lock";
	import Plus from "@lucide/svelte/icons/plus";

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
	let savedQuery = $state("");
	let editingNet = $state<SavedNetwork | null>(null);
	let adding = $state<SavedNetwork | null>(null);

	const VIRTUAL_IFACE_PREFIXES = ["veth", "docker", "br-", "virbr", "vnet", "tun", "tap", "vmnet", "podman"];
	let ethernetUp = $derived.by(() => {
		if (!ethernet) return false;
		const [iface, rawState] = ethernet.split(": ");
		if (!iface || VIRTUAL_IFACE_PREFIXES.some((prefix) => iface.startsWith(prefix))) return false;
		return rawState === "connected";
	});
	let activeNet = $derived(networks?.find((n) => n.active) ?? null);
	let nearby = $derived(networks?.filter((n) => !n.active) ?? []);
	let saved = $derived(crumbs.cfg?.networks ?? []);
	let shownSaved = $derived(saved.filter((n) => n.ssid.toLowerCase().includes(savedQuery.trim().toLowerCase())));

	// The one draft for every breadcrumbs section on this page (Places,
	// saved networks, advanced settings): they all edit `crumbs.cfg`.
	useDraft(() => crumbs.cfg, saveCrumbs, () => crumbs.cfg !== null);

	onMount(async () => {
		const info = await invoke<{ radio_enabled: boolean; ethernet: string | null }>("get_network_info");
		radioEnabled = info.radio_enabled;
		ethernet = info.ethernet;
		loadCrumbs().catch(() => {});
		if (info.radio_enabled) await scan();
	});

	async function toggleRadio(enabled: boolean) {
		const before = radioEnabled;
		radioEnabled = enabled;
		try {
			await recordChange(
				`Wi-Fi ${enabled ? "on" : "off"}`,
				() => invoke("set_wifi_radio", { enabled }),
				async () => { await invoke("set_wifi_radio", { enabled: before }); radioEnabled = before; },
			);
			if (enabled) setTimeout(scan, 1500);
			else networks = [];
		} catch (e) {
			radioEnabled = before;
			status = `Couldn't turn Wi-Fi ${enabled ? "on" : "off"}: ${e}`;
		}
	}

	async function scan() {
		scanning = true;
		try {
			networks = await invoke<WifiNetwork[]>("scan_wifi");
		} finally {
			scanning = false;
		}
	}

	async function connect(net: WifiNetwork) {
		if (net.secured && !net.known) {
			pendingSsid = pendingSsid === net.ssid ? null : net.ssid;
			password = "";
			return;
		}
		await join(net.ssid, null);
	}

	async function join(ssid: string, pw: string | null) {
		pendingSsid = null;
		password = "";
		status = `Connecting to ${ssid}…`;
		try {
			await invoke("connect_wifi", { ssid, password: pw });
			status = `Connected to ${ssid}`;
			await scan();
		} catch (e) {
			status = pw ? `Couldn't join ${ssid}. Check the password and try again.` : `Couldn't join ${ssid}: ${e}`;
		}
	}

	function strength(signal: number): string {
		return signal >= 75 ? "strong signal" : signal >= 50 ? "good signal" : signal >= 25 ? "weak signal" : "very weak signal";
	}
	function bars(signal: number): number {
		return signal >= 75 ? 4 : signal >= 50 ? 3 : signal >= 25 ? 2 : 1;
	}

	async function forget(net: SavedNetwork) {
		const cfg = crumbs.cfg!;
		const before = cfg.networks;
		await recordChange(
			`Forgot ${net.ssid}`,
			async () => { cfg.networks = before.filter((n) => n !== net); },
			async () => { cfg.networks = before; },
		);
	}

	function startAdd() {
		adding = { ssid: "", password: "", hidden: false };
	}
	function finishAdd() {
		const cfg = crumbs.cfg!;
		const net = adding!;
		const ssid = net.ssid.trim();
		if (!ssid) { adding = null; return; }
		const existing = cfg.networks.find((n) => n.ssid === ssid);
		if (existing) { existing.password = net.password; existing.hidden = net.hidden; }
		else cfg.networks = [...cfg.networks, { ...net, ssid }];
		adding = null;
	}
</script>

<ViewScaffold title="Wi-Fi">
	<div class="hero-card">
		<span class="big">
			{#if activeNet}<Wifi size={26} />{:else if ethernetUp}<Cable size={26} />{:else}<WifiOff size={26} />{/if}
		</span>
		<div>
			{#if activeNet}
				<h3>{activeNet.ssid}</h3>
				<p>Connected · {strength(activeNet.signal)} · {activeNet.secured ? "secured" : "open network"}{crumbs.current ? ` · ${crumbs.current}` : ""}</p>
			{:else if ethernetUp}
				<h3>Wired connection</h3>
				<p>Connected by cable{radioEnabled ? " · Wi-Fi is on" : ""}</p>
			{:else}
				<h3>Not connected</h3>
				<p>{radioEnabled ? "Pick a network below." : "Wi-Fi is off."}</p>
			{/if}
		</div>
		<div class="acts">
			<Switch bind:value={() => radioEnabled, (v) => toggleRadio(v)} ariaLabel="Wi-Fi" />
		</div>
	</div>

	<div data-tab="breadcrumbs"><Places nearbySsids={networks?.map((n) => n.ssid) ?? []} /></div>

	<Group title={scanning ? "Nearby networks · scanning…" : "Nearby networks"}>
		{#if !radioEnabled}
			<p class="empty">Turn on Wi-Fi to see networks.</p>
		{:else if networks === null}
			<p class="empty">Looking for networks…</p>
		{:else if nearby.length === 0}
			<p class="empty">No other networks found.</p>
		{:else}
			{#each nearby as net (net.ssid)}
				<div class="ui-item">
					<span class="bars" aria-label="{strength(net.signal)}">
						{#each [1, 2, 3, 4] as n (n)}<i class:lit={bars(net.signal) >= n}></i>{/each}
					</span>
					<div class="lb"><b>{net.ssid}</b><small>{net.known ? "Saved · " : ""}{net.secured ? "Secured" : "Open"}</small></div>
					{#if net.secured}<Lock size={14} class="lock" />{/if}
					<button class="btn small" onclick={() => connect(net)}>Connect</button>
				</div>
				{#if pendingSsid === net.ssid}
					<div class="pw-row">
						<!-- svelte-ignore a11y_autofocus -->
						<input class="ui-input" type="password" bind:value={password} placeholder="Password for {net.ssid}" autofocus onkeydown={(e) => e.key === "Enter" && join(net.ssid, password)} />
						<button class="btn primary small" onclick={() => join(net.ssid, password)}>Join</button>
					</div>
				{/if}
			{/each}
		{/if}
		{#if status}<p class="status" role="status">{status}</p>{/if}
		<div class="foot">
			<button class="btn small" disabled={scanning || !radioEnabled} onclick={scan}>{scanning ? "Scanning…" : "Scan again"}</button>
		</div>
		{#if crumbs.cfg}
			<details class="ui-disclosure">
				<summary>Saved networks ({saved.length})</summary>
				{#if saved.length > 6}<input class="ui-input search" bind:value={savedQuery} placeholder="Search {saved.length} saved networks" />{/if}
				{#each shownSaved as net (net)}
					<div class="ui-item">
						<div class="lb"><b>{net.ssid}</b>{#if net.hidden}<small>Hidden network</small>{/if}</div>
						<div class="acts">
							<button class="btn small ghost" onclick={() => (editingNet = net)}>Edit</button>
							<button class="btn small remove" onclick={() => forget(net)}>Forget</button>
						</div>
					</div>
				{/each}
				<div class="foot"><button class="btn small" onclick={startAdd}><Plus size={14} />Add a network</button></div>
			</details>
		{/if}
	</Group>
</ViewScaffold>

{#if editingNet}
	{@const net = editingNet}
	<Sheet title={net.ssid} onClose={() => (editingNet = null)}>
		<label class="field">
			<span>New password</span>
			<input class="ui-input" type="password" autocomplete="new-password" bind:value={net.password} placeholder="Leave empty to keep the current one" />
		</label>
		<div class="toggle">
			<div><b>Hidden network</b><small>It doesn't announce its name, so BOS asks for it by name.</small></div>
			<Switch bind:value={net.hidden} ariaLabel="Hidden network" />
		</div>
	</Sheet>
{/if}

{#if adding}
	<Sheet title="Add a network" lede="For a network that isn't nearby right now, or one that hides its name." onClose={() => (adding = null)}>
		<label class="field"><span>Network name</span><input class="ui-input" bind:value={adding.ssid} placeholder="Exactly as it is spelled" /></label>
		<label class="field"><span>Password</span><input class="ui-input" type="password" autocomplete="new-password" bind:value={adding.password} placeholder="Leave empty for an open network" /></label>
		<div class="toggle">
			<div><b>Hidden network</b><small>It doesn't announce its name.</small></div>
			<Switch bind:value={adding.hidden} ariaLabel="Hidden network" />
		</div>
		{#snippet actions()}
			<button class="btn" onclick={() => (adding = null)}>Cancel</button>
			<button class="btn primary" disabled={!adding?.ssid.trim()} onclick={finishAdd}>Add</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.empty { color:var(--muted); font-size:12.5px; margin:6px 2px 10px; }
	.bars { display:inline-flex; gap:2px; align-items:flex-end; height:13px; flex:none; }
	.bars i { width:3px; border-radius:1px; background:color-mix(in srgb, var(--fg) 22%, transparent); }
	.bars i:nth-child(1) { height:4px; } .bars i:nth-child(2) { height:7px; } .bars i:nth-child(3) { height:10px; } .bars i:nth-child(4) { height:13px; }
	.bars i.lit { background:var(--accent); }
	.ui-item :global(.lock) { color:var(--muted); flex:none; }
	.pw-row { display:flex; gap:8px; padding:0 2px 10px; }
	.pw-row input { flex:1; }
	.status { margin:6px 2px; color:var(--muted); font-size:12px; }
	.foot { display:flex; gap:8px; padding:4px 0 10px; }
	.foot .btn { display:inline-flex; align-items:center; gap:6px; }
	.search { width:100%; margin:2px 0 6px; }
	.field { display:flex; flex-direction:column; gap:6px; padding:10px 0; font-size:13px; }
	.toggle { display:flex; align-items:center; gap:12px; padding:10px 0; border-top:1px solid var(--line); }
	.toggle > div { flex:1; min-width:0; }
	.toggle b { display:block; font-weight:400; }
	.toggle small { display:block; color:var(--muted); font-size:12px; margin-top:2px; }
</style>
