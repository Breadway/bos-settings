<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface BreadlockConfig {
		background_mode: string;
		background_path: string;
		background_blur: boolean;
		clock_format: string;
		font_family: string;
		fail_timeout_ms: number;
	}

	let cfg = $state<BreadlockConfig | null>(null);

	// Plain choices for the raw breadlock.toml values; anything custom stays selectable.
	const CLOCKS: Record<string, string> = { "%H:%M": "24-hour", "%H:%M:%S": "24-hour with seconds", "%I:%M %p": "12-hour", "%I:%M:%S %p": "12-hour with seconds" };
	const clockOptions = (cur: string) => Array.from(new Set([...Object.keys(CLOCKS), cur].filter(Boolean)));
	const clockLabels = (cur: string) => ({ ...(cur && !(cur in CLOCKS) ? { [cur]: `Custom (${cur})` } : {}), ...CLOCKS });
	const WAITS = [0, 500, 800, 1000, 2000, 3000];
	const waitLabel = (ms: number) => (ms === 0 ? "Don't wait" : ms < 1000 ? `${ms / 1000} seconds`.replace(/^0\.5 seconds$/, "Half a second") : ms === 1000 ? "1 second" : `${ms / 1000} seconds`);
	const waitOptions = (cur: number) => Array.from(new Set([...WAITS, cur])).sort((a, b) => a - b).map(String);
	const waitLabels = (cur: number) => Object.fromEntries([...WAITS, cur].map((ms) => [String(ms), waitLabel(ms)]));
	let examplePath = $state<string | null>(null);

	onMount(async () => {
		cfg = await invoke<BreadlockConfig>("get_breadlock_config");
		examplePath = await invoke<string | null>("breadlock_example_path");
	});

	async function save() {
		await invoke("save_breadlock_config", { cfg });
	}
	useDraft(() => (cfg), save, () => cfg !== null);
</script>

<ViewScaffold title="Lock">
	<Group
		title="How locking works"
		hint="Login and PAM are packaged. This only styles the lock screen."
	>
		<Hint text="Super+L locks. Login screen is packaged separately." />
		<button class="primary" onclick={() => invoke("lock_session")}>Lock now</button>
	</Group>

	{#if cfg}
		<Group title="Lock screen">
			<SelectField label="Background" bind:value={cfg.background_mode} options={["color", "image"]} labels={{ color: "Solid colour", image: "Picture" }} />
			{#if cfg.background_mode === "image"}
				<FileField label="Image" bind:value={cfg.background_path} placeholder="PNG, cover-fit" extensions={["png"]} />
			{/if}
			<SwitchField label="Blur background" bind:value={cfg.background_blur} />
			<Hint text="The lock screen doesn't draw blur yet. This setting is kept for when it does." />
			<SelectField label="Clock" bind:value={cfg.clock_format} options={clockOptions(cfg.clock_format)} labels={clockLabels(cfg.clock_format)} />
			<details class="adv"><summary>Custom clock format</summary><TextField label="Format" bind:value={cfg.clock_format} placeholder="%H:%M" /></details>
			<TextField label="Font" bind:value={cfg.font_family} placeholder="Varela Round" />
			<SelectField label="After a wrong password, wait" bind:value={() => String(cfg!.fail_timeout_ms), (v) => (cfg!.fail_timeout_ms = Number(v))} options={waitOptions(cfg.fail_timeout_ms)} labels={waitLabels(cfg.fail_timeout_ms)} />
		</Group>
	{/if}

	<Group title="Files">
		<button class="secondary" onclick={() => invoke("open_breadlock_config")}>Open breadlock.toml in editor</button>
		{#if examplePath}
			<button class="secondary" onclick={() => invoke("open_breadlock_example")}>Open example ({examplePath})</button>
		{/if}
	</Group>
</ViewScaffold>

<style>
	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
		align-self: flex-start;
		margin-top: var(--space-xs, 4px);
	}

	.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	.secondary {
		background-color: var(--surface);
		color: var(--on-surface);
	}
</style>
