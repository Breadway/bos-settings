<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import Group from "$lib/components/Group.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import ChipPickerField from "$lib/components/ChipPickerField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import Switch from "$lib/components/Switch.svelte";

	interface Settings {
		default_profile: string;
		dns: string;
		exit_node: string;
		ping_host: string;
		connectivity_url: string;
		nmcli_wait: number;
		watch_interval: number;
	}
	interface Network {
		ssid: string;
		// Write-only. The backend never returns a stored PSK; empty means
		// "keep the on-disk secret / let NetworkManager remember."
		password?: string | null;
		hidden: boolean;
	}
	interface Profile {
		name: string;
		networks: string[];
		detect_ssids: string[];
		bootstrap: string;
		exit_node: string;
		tailscale: boolean;
		include_all_known: boolean;
	}
	interface BreadcrumbsConfig {
		settings: Settings;
		networks: Network[];
		profiles: Profile[];
	}

	let cfg = $state<BreadcrumbsConfig | null>(null);
	// Profile names are free-text (set via "Add profile" below), not a fixed
	// enum  -  populate the default-profile options from whatever profiles
	// actually exist, rather than a hardcoded guess that can't match a
	// user's real profile names.
	let profileNames = $derived(cfg?.profiles.map((p) => p.name) ?? []);
	// Saved network SSIDs feed the per-profile network pickers below  -  no
	// typing an SSID by hand and hoping it matches one saved above.
	let savedSsids = $derived(cfg?.networks.map((n) => n.ssid).filter((s) => s.trim().length > 0) ?? []);

	onMount(async () => {
		const loaded = await invoke<BreadcrumbsConfig>("get_breadcrumbs_config");
		loaded.networks = loaded.networks.map((n) => ({ ...n, password: "" }));
		cfg = loaded;
	});

	function addNetwork() {
		cfg!.networks = [...cfg!.networks, { ssid: "", password: "", hidden: false }];
	}
	function removeNetwork(i: number) {
		cfg!.networks = cfg!.networks.filter((_, idx) => idx !== i);
	}
	function addProfile() {
		cfg!.profiles = [
			...cfg!.profiles,
			{ name: "new", networks: [], detect_ssids: [], bootstrap: "", exit_node: "", tailscale: false, include_all_known: false },
		];
	}
	function removeProfile(i: number) {
		cfg!.profiles = cfg!.profiles.filter((_, idx) => idx !== i);
	}

	async function save() {
		const payload = {
			...cfg!,
			networks: cfg!.networks.map((n) => ({
				ssid: n.ssid,
				hidden: n.hidden,
				// Omit empty so the backend treats it as "keep / let NM remember"
				// rather than writing password = "".
				...(n.password && n.password.length > 0 ? { password: n.password } : {}),
			})),
		};
		await invoke("save_breadcrumbs_config", { input: payload });
		// Clear typed secrets from the UI after a successful save so a later
		// glance at the field doesn't look like a stored PSK came back.
		cfg!.networks = cfg!.networks.map((n) => ({ ...n, password: "" }));
	}
	useDraft(() => (cfg), save, () => cfg !== null);

	// One sentence per place so the profile list reads like the mockup's "At school: ..." lines.
	function list(xs: string[] | undefined): string {
		const v = (xs ?? []).filter(Boolean);
		return v.length <= 2 ? v.join(" or ") : `${v.slice(0, -1).join(", ")} or ${v[v.length - 1]}`;
	}
	function placeSummary(p: { networks?: string[]; detect_ssids?: string[]; exit_node?: string; tailscale?: boolean; include_all_known?: boolean; bootstrap?: string }): string {
		const parts: string[] = [];
		if (p.networks?.length) parts.push(`joins ${list(p.networks)}${p.include_all_known ? " (or any saved network)" : ""}`);
		else if (p.include_all_known) parts.push("joins any saved network");
		else parts.push("has no networks yet");
		if (p.bootstrap) parts.push(`signs in through ${p.bootstrap} first`);
		if (p.detect_ssids?.length) parts.push(`is picked automatically when ${list(p.detect_ssids)} is nearby`);
		if (p.exit_node) parts.push(`sends traffic through ${p.exit_node}`);
		else if (p.tailscale) parts.push("turns on Tailscale");
		return parts.length > 1 ? `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}.` : `${parts[0]}.`;
	}
	// breadcrumbs used to backfill lowercase home/work/away next to a user's Home/Work/Away.
	function isEmptyTwin(p: { name: string; networks?: string[]; detect_ssids?: string[] }, i: number): string | null {
		if ((p.networks?.length ?? 0) || (p.detect_ssids?.length ?? 0)) return null;
		const other = cfg!.profiles.find((q, j) => j !== i && q.name !== p.name && q.name.toLowerCase() === p.name.toLowerCase());
		return other ? other.name : null;
	}
</script>

<ViewScaffold title="Wi-Fi Profiles">
	<ServiceControl unit="breadcrumbs.service" hasConfig />

	{#if cfg}
		<Group title="Settings">
			{#if profileNames.length > 0}
				<SelectField label="Default profile" bind:value={cfg.settings.default_profile} options={profileNames} />
			{:else}
				<TextField label="Default profile" bind:value={cfg.settings.default_profile} placeholder="add a profile below first" />
			{/if}
			<TextField label="DNS" bind:value={cfg.settings.dns} placeholder="1.1.1.1" />
			<TextField label="Exit node" bind:value={cfg.settings.exit_node} placeholder="tailscale exit node" />
			<TextField label="Ping host" bind:value={cfg.settings.ping_host} placeholder="1.1.1.1" />
			<TextField label="Connectivity check URL" bind:value={cfg.settings.connectivity_url} />
			<NumberField label="Connection timeout (s)" bind:value={cfg.settings.nmcli_wait} min={1} max={120} />
			<NumberField label="Check connectivity every (s)" bind:value={cfg.settings.watch_interval} min={1} max={600} />
		</Group>

		<Group
			title="Saved networks"
			hint="Leave password blank to keep the saved one."
			wide
		>
			<div class="list">
				{#each cfg.networks as net, i (i)}
					<div class="net-row">
						<input type="text" bind:value={net.ssid} placeholder="Network name (SSID)" class="ssid" />
						<input
							type="password"
							autocomplete="new-password"
							bind:value={net.password}
							placeholder="New password (blank = keep / NM remembers)"
							class="pass"
						/>
						<div class="hidden-label">
							<Switch bind:value={net.hidden} ariaLabel="Hidden network" />
							<button type="button" class="label-text" onclick={() => (net.hidden = !net.hidden)}>Hidden network</button>
						</div>
						<button class="remove" onclick={() => removeNetwork(i)}>Remove</button>
					</div>
				{/each}
			</div>
			<button class="add" onclick={addNetwork}>Add network</button>
		</Group>

		<Group title="Places" hint="Each place picks which networks to join and how traffic is routed when you are there." wide>
			{#each cfg.profiles as profile, i (i)}
				{@const twin = isEmptyTwin(profile, i)}
				<div class="profile-card">
					<p class="summary"><b>{profile.name || "Unnamed place"}</b> {placeSummary(profile)}</p>
					{#if twin}<p class="twin">This place is empty and has the same name as {twin}. It was added by an older version of breadcrumbs; you can remove it.</p>{/if}
					<details open={!profile.name || profile.name === "new"}>
						<summary>Edit {profile.name || "place"}</summary>
					<div class="profile-header">
						<input type="text" bind:value={profile.name} placeholder="Profile name (e.g. Home)" class="profile-name" />
						<button class="remove" onclick={() => removeProfile(i)}>Remove</button>
					</div>

					<ChipPickerField
						label="Networks in this profile"
						bind:value={profile.networks}
						options={savedSsids}
						emptyOptionsHint="Add a saved network above first."
					/>
					<ChipPickerField
						label="Auto-switch here when nearby"
						bind:value={profile.detect_ssids}
						options={savedSsids}
						emptyOptionsHint="Add a saved network above first."
					/>

					<label class="field-label">
						Exit node
						<input type="text" bind:value={profile.exit_node} placeholder="Tailscale exit node (optional)" />
					</label>
					<label class="field-label">
						Connect through first
						<select bind:value={profile.bootstrap}>
							<option value="">Nothing (connect directly)</option>
							{#each Array.from(new Set([...savedSsids, ...(profile.bootstrap ? [profile.bootstrap] : [])])) as ssid (ssid)}<option value={ssid}>{ssid}</option>{/each}
						</select>
						<small>A saved network to join first, for example a sign-in network, before switching to this place's networks.</small>
					</label>
					<div class="check-label">
						<Switch bind:value={profile.tailscale} ariaLabel="Enable Tailscale" />
						<button type="button" class="label-text" onclick={() => (profile.tailscale = !profile.tailscale)}>Enable Tailscale</button>
					</div>
					<div class="check-label">
						<Switch bind:value={profile.include_all_known} ariaLabel="Also allow any other saved network" />
						<button
							type="button"
							class="label-text"
							onclick={() => (profile.include_all_known = !profile.include_all_known)}
						>
							Also allow any other saved network, not just the ones listed above
						</button>
					</div>
					</details>
				</div>
			{/each}
			<button class="add" onclick={addProfile}>Add profile</button>
		</Group>

		<SaveButton onSave={save} />
	{/if}
</ViewScaffold>

<style>
	.summary { margin: 0 0 6px; font-size: 13px; color: var(--muted); }
	.summary b { color: var(--fg); font-weight: 500; }
	.twin { margin: 0 0 6px; font-size: 12px; color: var(--warn, #d6b25e); }
	.profile-card details > summary { cursor: pointer; color: var(--muted); font-size: 12.5px; padding: 4px 0 8px; }
	.field-label small { color: var(--muted); font-size: 11.5px; }
	.list {
		display: flex;
		flex-direction: column;
		gap: var(--space-xs, 4px);
		margin-bottom: var(--space-sm, 8px);
	}

	.net-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm, 8px);
		align-items: center;
		background-color: var(--surface);
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.ssid {
		width: 20ch;
	}

	.pass {
		width: 14ch;
	}

	.hidden-label,
	.check-label {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.85;
	}

	.label-text {
		background: transparent;
		border: none;
		padding: 0;
		margin: 0;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}

	input[type="text"],
	input[type="password"] {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.remove {
		background-color: transparent;
		border: 1px solid color-mix(in oklab, var(--red) 45%, transparent);
		color: var(--red);
	}

	.add {
		background-color: var(--surface);
		color: var(--on-surface);
		margin-top: var(--space-sm, 8px);
		align-self: flex-start;
	}

	.profile-card {
		background-color: var(--surface);
		color: var(--on-surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px);
		margin-bottom: var(--space-sm, 8px);
		display: flex;
		flex-direction: column;
		gap: var(--space-xs, 4px);
	}

	.profile-header {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-bottom: 4px;
	}

	.profile-name {
		flex: 1;
		font-weight: 500;
	}

	.field-label {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
	}

	.field-label input[type="text"] {
		flex: 1;
	}
</style>
