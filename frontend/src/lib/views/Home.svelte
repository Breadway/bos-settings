<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { go, nav } from "$lib/nav.svelte";
	import { searchSettings, type SearchHit } from "$lib/search";
	import Search from "@lucide/svelte/icons/search";
	import Wifi from "@lucide/svelte/icons/wifi";
	import BatteryFull from "@lucide/svelte/icons/battery-full";
	import Download from "@lucide/svelte/icons/download";
	import Monitor from "@lucide/svelte/icons/monitor";
	import Palette from "@lucide/svelte/icons/palette";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import Keyboard from "@lucide/svelte/icons/keyboard";
	import AppWindow from "@lucide/svelte/icons/app-window";
	import Shield from "@lucide/svelte/icons/shield";
	import Package from "@lucide/svelte/icons/package";

	let query = $state("");
	let searchEl: HTMLInputElement | undefined = $state();
	let hits = $derived(query.trim() ? searchSettings(query) : []);

	let wifiLabel = $state("Wi-Fi");
	let wifiDetail = $state("Checking…");
	let batteryLabel = $state("Battery");
	let batteryDetail = $state("Checking…");
	let updateLabel = $state("Updates");
	let updateDetail = $state("Checking…");

	const tiles: { id: string; tab?: string; title: string; desc: string; icon: typeof Wifi }[] = [
		{ id: "network", title: "Wi-Fi & internet", desc: "Radio, saved networks, VPN.", icon: Wifi },
		{ id: "displays", title: "Displays", desc: "Arrange, scale, night light.", icon: Monitor },
		{ id: "appearance", title: "Appearance", desc: "Wallpaper drives the palette.", icon: Palette },
		{ id: "sound", title: "Sound", desc: "Devices, volume, mute.", icon: Volume2 },
		{ id: "power", title: "Power", desc: "Brightness, charge limit, battery.", icon: BatteryFull },
		{ id: "input", title: "Keyboard & mouse", desc: "Layouts, keybinds, tap-to-click.", icon: Keyboard },
		{ id: "desktop", title: "Bar & apps", desc: "Bar, launcher, lock, screenshots.", icon: AppWindow },
		{ id: "privacy", title: "Privacy & users", desc: "Firewall and accounts.", icon: Shield },
		{ id: "system", title: "Updates", desc: "Bakery, pacman, snapshots, restic.", icon: Package },
	];

	$effect(() => {
		if (nav.searchNonce > 0) {
			searchEl?.focus();
			searchEl?.select();
		}
	});

	onMount(async () => {
		try {
			const net = await invoke<{ radio_enabled: boolean; ethernet: string | null }>("get_network_info");
			if (!net.radio_enabled) {
				wifiLabel = "Wi-Fi off";
				wifiDetail = "Radio disabled";
			} else {
				const networks = await invoke<{ ssid: string; active: boolean; signal: number }[]>("scan_wifi").catch(
					() => [],
				);
				const active = networks.find((n) => n.active);
				if (active) {
					wifiLabel = active.ssid;
					wifiDetail = `Wi-Fi · ${active.signal}%`;
				} else {
					wifiLabel = "Wi-Fi";
					wifiDetail = net.ethernet ? "On · no network" : "On";
				}
			}
		} catch {
			wifiDetail = "Unavailable";
		}

		try {
			const power = await invoke<{ battery: [string, string][]; power_source: string }>("get_power_info");
			const charge = power.battery.find(([k]) => k === "Charge")?.[1] ?? "";
			const remain = power.battery.find(([k]) => k === "Time remaining")?.[1];
			batteryLabel = charge || power.power_source;
			batteryDetail = remain ? `${remain}` : power.power_source;
		} catch {
			batteryDetail = "Unavailable";
		}

		try {
			const st = await invoke<{
				pacman: unknown[];
				bakery: unknown[];
				firmware: unknown[];
			}>("get_updates_status");
			const n = st.pacman.length + st.bakery.length + st.firmware.length;
			updateLabel = n === 0 ? "Up to date" : `${n} update${n === 1 ? "" : "s"}`;
			updateDetail = n === 0 ? "bakery + pacman" : "bakery + pacman + firmware";
		} catch {
			updateDetail = "Unavailable";
		}
	});

	function goHit(hit: SearchHit) {
		go(hit.page, hit.tab);
	}
</script>

<div class="home">
	<h1>Settings</h1>
	<p class="lede">Wi-Fi, lock screen, updates, keybinds.</p>

	<div class="search-wrap">
		<Search size={18} />
		<input
			bind:this={searchEl}
			class="bigsearch"
			bind:value={query}
			placeholder="Try “night light”, “keybinds”, “forget network”…"
		/>
	</div>

	{#if hits.length > 0}
		<div class="results">
			{#each hits as hit (`${hit.page}:${hit.tab ?? ""}:${hit.label}`)}
				<button class="hit" onclick={() => goHit(hit)}>
					<span>{hit.label}</span>
					<em>{hit.tab ?? hit.page}</em>
				</button>
			{/each}
		</div>
	{:else}
		<div class="status">
			<button class="stat" onclick={() => go("network")}>
				<Wifi size={18} />
				<div><b>{wifiLabel}</b><span>{wifiDetail}</span></div>
			</button>
			<button class="stat" onclick={() => go("power")}>
				<BatteryFull size={18} />
				<div><b>{batteryLabel}</b><span>{batteryDetail}</span></div>
			</button>
			<button class="stat" onclick={() => go("system")}>
				<Download size={18} />
				<div><b>{updateLabel}</b><span>{updateDetail}</span></div>
			</button>
		</div>

		<div class="tiles">
			{#each tiles as tile (tile.id + (tile.tab ?? ""))}
				<button class="tile" onclick={() => go(tile.id, tile.tab)}>
					<div class="ico"><tile.icon size={18} /></div>
					<div class="t">{tile.title}</div>
					<div class="d">{tile.desc}</div>
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.home {
		padding: 28px 36px 56px;
		max-width: 1080px;
		margin-inline: auto;
	}

	h1 {
		font-size: 28px;
		font-weight: 600;
		letter-spacing: -0.04em;
		margin: 0;
	}

	.lede {
		margin: 6px 0 0;
		color: var(--muted);
		font-size: 14px;
	}

	.search-wrap {
		position: relative;
		margin-top: 22px;
	}

	.search-wrap :global(svg) {
		position: absolute;
		left: 16px;
		top: 16px;
		opacity: 0.45;
		pointer-events: none;
	}

	.bigsearch {
		width: 100%;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 14px 16px 14px 44px;
		font-size: 16px;
	}

	.status {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
		margin-top: 18px;
	}

	.stat {
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 14px 16px;
		display: flex;
		gap: 12px;
		align-items: center;
		text-align: left;
		cursor: pointer;
		color: inherit;
	}

	.stat :global(svg) {
		color: var(--accent);
		flex-shrink: 0;
	}

	.stat b {
		display: block;
		font-size: 14px;
		font-weight: 600;
	}

	.stat span {
		font-size: 12px;
		color: var(--muted);
	}

	.tiles {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 12px;
		margin-top: 18px;
	}

	.tile {
		text-align: left;
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 14px;
		padding: 16px 16px 14px;
		min-height: 108px;
		display: flex;
		flex-direction: column;
		gap: 8px;
		cursor: pointer;
		color: inherit;
		transition:
			transform 0.2s cubic-bezier(0.22, 1, 0.36, 1),
			border-color 0.15s;
	}

	.tile:hover {
		transform: translateY(-2px);
		border-color: color-mix(in srgb, var(--fg) 18%, transparent);
	}

	.ico {
		width: 34px;
		height: 34px;
		border-radius: 10px;
		display: grid;
		place-items: center;
		background: color-mix(in oklab, var(--fg) 9%, transparent);
		color: var(--fg);
	}

	.t {
		font-size: 14.5px;
		font-weight: 600;
	}

	.d {
		font-size: 12px;
		color: var(--muted);
		line-height: 1.35;
	}

	.results {
		margin-top: 12px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.hit {
		text-align: left;
		padding: 10px 12px;
		border-radius: 10px;
		background: var(--surface);
		border: 1px solid var(--line);
		display: flex;
		justify-content: space-between;
		gap: 12px;
		cursor: pointer;
		color: inherit;
	}

	.hit:hover {
		background: var(--surface-2, var(--surface));
	}

	.hit em {
		font-style: normal;
		color: var(--muted);
		font-size: 12px;
	}

	@media (max-width: 800px) {
		.status,
		.tiles {
			grid-template-columns: 1fr;
		}
	}
</style>
