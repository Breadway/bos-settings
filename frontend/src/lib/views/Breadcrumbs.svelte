<script lang="ts">
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
		password: string;
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
	// enum — populate the default-profile options from whatever profiles
	// actually exist, rather than a hardcoded guess that can't match a
	// user's real profile names.
	let profileNames = $derived(cfg?.profiles.map((p) => p.name) ?? []);
	// Saved network SSIDs feed the per-profile network pickers below — no
	// typing an SSID by hand and hoping it matches one saved above.
	let savedSsids = $derived(cfg?.networks.map((n) => n.ssid).filter((s) => s.trim().length > 0) ?? []);

	onMount(async () => {
		cfg = await invoke<BreadcrumbsConfig>("get_breadcrumbs_config");
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
		await invoke("save_breadcrumbs_config", { input: cfg });
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

		<Group title="Saved networks" wide>
			<div class="list">
				{#each cfg.networks as net, i (i)}
					<div class="net-row">
						<input type="text" bind:value={net.ssid} placeholder="Network name (SSID)" class="ssid" />
						<input type="password" bind:value={net.password} placeholder="Password" class="pass" />
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

		<Group title="Location profiles" hint="Each profile switches DNS/exit-node/routing when you're on one of its networks." wide>
			{#each cfg.profiles as profile, i (i)}
				<div class="profile-card">
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
						Run on connect
						<input type="text" bind:value={profile.bootstrap} placeholder="Optional command (optional)" />
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
				</div>
			{/each}
			<button class="add" onclick={addProfile}>Add profile</button>
		</Group>

		<SaveButton onSave={save} />
	{/if}
</ViewScaffold>

<style>
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
		background-color: var(--red);
		color: var(--on-red);
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
