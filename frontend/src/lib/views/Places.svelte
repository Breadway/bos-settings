<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import Group from "$lib/components/Group.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Switch from "$lib/components/Switch.svelte";
	import ChipPickerField from "$lib/components/ChipPickerField.svelte";
	import { crumbs, isLeftoverPlace, placeSentence, unusedTailscaleBits, type Place } from "$lib/breadcrumbs.svelte";
	import { recordChange } from "$lib/changes.svelte";
	import Home from "@lucide/svelte/icons/house";
	import MapPin from "@lucide/svelte/icons/map-pin";
	import Globe from "@lucide/svelte/icons/globe";
	import Plus from "@lucide/svelte/icons/plus";
	import Info from "@lucide/svelte/icons/info";

	let { nearbySsids = [] }: { nearbySsids?: string[] } = $props();

	let editing = $state<Place | null>(null);
	let draftName = $state("");
	let switching = $state<string | null>(null);
	let message = $state("");

	let cfg = $derived(crumbs.cfg);
	let leftovers = $derived(cfg ? cfg.profiles.filter((p) => isLeftoverPlace(p, cfg, crumbs.current)) : []);
	let places = $derived(cfg ? cfg.profiles.filter((p) => !leftovers.includes(p)) : []);
	let savedSsids = $derived(cfg?.networks.map((n) => n.ssid).filter(Boolean) ?? []);
	let detectOptions = $derived([...new Set([...savedSsids, ...nearbySsids])]);

	function icon(name: string) {
		const n = name.toLowerCase();
		if (n.includes("home")) return Home;
		if (n.includes("away") || n.includes("travel")) return Globe;
		return MapPin;
	}

	async function use(place: Place) {
		if (place.name === crumbs.current || switching) return;
		switching = place.name;
		message = `Switching to ${place.name}…`;
		try {
			await invoke("set_current_place", { name: place.name });
			crumbs.current = place.name;
			message = `Now using ${place.name}`;
		} catch (e) {
			message = `Couldn't switch to ${place.name}: ${e}`;
		} finally {
			switching = null;
		}
	}

	function edit(place: Place) {
		editing = place;
		draftName = place.name;
	}

	function commitName() {
		if (!editing || !cfg) return;
		const name = draftName.trim();
		if (!name || name === editing.name) { draftName = editing.name; return; }
		if (cfg.profiles.some((p) => p !== editing && p.name === name)) { draftName = editing.name; message = `There is already a place called ${name}.`; return; }
		if (cfg.settings.default_profile === editing.name) cfg.settings.default_profile = name;
		editing.name = name;
	}

	function addPlace() {
		if (!cfg) return;
		let name = "New place";
		for (let i = 2; cfg.profiles.some((p) => p.name === name); i++) name = `New place ${i}`;
		const place: Place = { name, networks: [], detect_ssids: [], bootstrap: "", exit_node: "", tailscale: false, include_all_known: false };
		cfg.profiles = [...cfg.profiles, place];
		edit(cfg.profiles[cfg.profiles.length - 1]);
	}

	async function removePlace(place: Place) {
		if (!cfg) return;
		const before = cfg.profiles;
		editing = null;
		await recordChange(
			`Removed ${place.name}`,
			async () => { cfg.profiles = before.filter((p) => p !== place); },
			async () => { cfg.profiles = before; },
		);
	}

	async function tidyUp() {
		if (!cfg) return;
		const before = cfg.profiles;
		const gone = leftovers.map((p) => p.name);
		await recordChange(
			`Removed ${gone.join(", ")}`,
			async () => { cfg.profiles = before.filter((p) => !gone.includes(p.name)); },
			async () => { cfg.profiles = before; },
		);
	}
</script>

{#if cfg}
	<Group title="Places" hint="A place is a set of networks and rules. BOS picks the right one when it sees a familiar network nearby.">
		<div class="ui-chips" data-setting="Places">
			{#each places as place (place)}
				{@const Icon = icon(place.name)}
				<button class="ui-chip" class:on={place.name === crumbs.current} disabled={switching !== null} title={place.name === crumbs.current ? "In use now" : `Switch to ${place.name} now`} onclick={() => use(place)}>
					<Icon size={14} />{place.name}
				</button>
			{/each}
			<button class="ui-chip" onclick={addPlace}><Plus size={14} />Add a place</button>
		</div>
		{#if message}<p class="msg" role="status">{message}</p>{/if}
		{#each places as place (place)}
			<div class="ui-item">
				<div class="lb">
					<b>{place.name}{#if place.name === crumbs.current}<span class="ui-tag ok">In use</span>{/if}{#if place.name === cfg.settings.default_profile}<span class="ui-tag">Fallback</span>{/if}</b>
					<small>{placeSentence(place, cfg.settings.exit_node)}</small>
					{#if unusedTailscaleBits(place)}<small class="note">{unusedTailscaleBits(place)}</small>{/if}
				</div>
				<button class="btn small" onclick={() => edit(place)}>Edit</button>
			</div>
		{/each}
		{#if leftovers.length}
			<div class="warnbox">
				<Info size={16} />
				<span>You also have empty {leftovers.map((p) => p.name).join(", ")} {leftovers.length === 1 ? "place" : "places"} left over from an older version.</span>
				<button class="btn small" onclick={tidyUp}>Tidy up</button>
			</div>
		{/if}
	</Group>
{/if}

{#if editing && cfg}
	{@const place = editing}
	<Sheet title="Edit {place.name}" onClose={() => { commitName(); editing = null; }}>
		<label class="field">
			<span>Name</span>
			<input class="ui-input" bind:value={draftName} onblur={commitName} onkeydown={(e) => e.key === "Enter" && commitName()} />
		</label>
		<ChipPickerField label="Join these networks, in this order" bind:value={place.networks} options={savedSsids} emptyOptionsHint="Save a network first." />
		<div class="toggle">
			<div><b>Also try any other saved network</b><small>When none of the networks above are around.</small></div>
			<Switch bind:value={place.include_all_known} ariaLabel="Also try any other saved network" />
		</div>
		<ChipPickerField label="Recognise this place when these are nearby" bind:value={place.detect_ssids} options={detectOptions} emptyOptionsHint="No networks to pick from yet." />
		<div class="toggle">
			<div><b>Send traffic through Tailscale</b><small>Waits for Tailscale before using the network, for example where sites are filtered.</small></div>
			<Switch bind:value={place.tailscale} ariaLabel="Send traffic through Tailscale" />
		</div>
		{#if place.tailscale}
			<label class="field">
				<span>Exit node</span>
				<input class="ui-input" bind:value={place.exit_node} placeholder={cfg.settings.exit_node ? `${cfg.settings.exit_node} (the usual one)` : "Name of a Tailscale exit node"} />
			</label>
			<label class="field">
				<span>Sign in through first</span>
				<select class="ui-input" bind:value={place.bootstrap}>
					<option value="">Nothing, connect directly</option>
					{#each [...new Set([...savedSsids, ...(place.bootstrap ? [place.bootstrap] : [])])] as ssid (ssid)}<option value={ssid}>{ssid}</option>{/each}
				</select>
				<small>A network BOS joins first so Tailscale can start, before it moves to the networks above.</small>
			</label>
		{/if}
		<div class="toggle">
			<div><b>Use when no place is recognised</b><small>The fallback place.</small></div>
			<Switch bind:value={() => cfg.settings.default_profile === place.name, (v) => { if (v) cfg.settings.default_profile = place.name; }} disabled={cfg.settings.default_profile === place.name} ariaLabel="Use when no place is recognised" />
		</div>
		{#snippet actions()}
			<button class="btn remove" disabled={cfg.settings.default_profile === place.name} title={cfg.settings.default_profile === place.name ? "Pick another fallback place first" : undefined} onclick={() => removePlace(place)}>Remove place</button>
			<span class="spacer"></span>
			<button class="btn primary" onclick={() => { commitName(); editing = null; }}>Done</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.ui-item .note { color:#d6b25e; }
	.ui-item b :global(.ui-tag) { margin-left:6px; vertical-align:1px; }
	.msg { margin:0 0 6px; font-size:12px; color:var(--muted); }
	.field { display:flex; flex-direction:column; gap:6px; padding:10px 0; font-size:13px; }
	.field small { color:var(--muted); font-size:12px; }
	.toggle { display:flex; align-items:center; gap:12px; padding:10px 0; border-top:1px solid var(--line); }
	.toggle > div { flex:1; min-width:0; }
	.toggle b { display:block; font-weight:400; }
	.toggle small { display:block; color:var(--muted); font-size:12px; margin-top:2px; }
	.spacer { flex:1; }
</style>
