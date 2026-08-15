<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	interface BreadlockConfig {
		background_mode: string;
		background_path: string;
		background_blur: boolean;
		clock_format: string;
		font_family: string;
		fail_timeout_ms: number;
	}

	let cfg = $state<BreadlockConfig | null>(null);
	let examplePath = $state<string | null>(null);

	onMount(async () => {
		cfg = await invoke<BreadlockConfig>("get_breadlock_config");
		examplePath = await invoke<string | null>("breadlock_example_path");
	});

	async function save() {
		await invoke("save_breadlock_config", { cfg });
	}
</script>

<ViewScaffold title="Lock & greet">
	<Group
		title="How locking works"
		hint="This panel does not configure PAM. Authentication is the breadlock PAM service (/etc/pam.d/breadlock) plus greetd for login — both are packaged, not user-editable from Settings."
	>
		<Hint
			text="Super+L runs loginctl lock-session. hypridle's lock_cmd / idle listener then starts breadlock, which owns the already-running Hyprland session via ext-session-lock-v1."
		/>
		<Hint
			text="breadgreet is the graphical greetd greeter (replacing tuigreet). Its live config is typically /etc/greetd/breadgreet.toml, owned by the greeter user — not written from here."
		/>
		<button class="primary" onclick={() => invoke("lock_session")}>Lock now</button>
	</Group>

	{#if cfg}
		<Group title="Lock screen" hint="~/.config/breadlock/breadlock.toml — every field is optional; breadlock runs with these defaults if the file is missing.">
			<SelectField label="Background" bind:value={cfg.background_mode} options={["color", "image"]} />
			{#if cfg.background_mode === "image"}
				<FileField label="Image" bind:value={cfg.background_path} placeholder="PNG, cover-fit" extensions={["png"]} />
			{/if}
			<SwitchField label="Blur background" bind:value={cfg.background_blur} />
			<Hint text="Blur is accepted in the file but not implemented yet (needs a wlr-screencopy capture). breadlock logs a warning and shows the background unblurred." />
			<TextField label="Clock format" bind:value={cfg.clock_format} placeholder="%H:%M" />
			<TextField label="Font" bind:value={cfg.font_family} placeholder="Varela Round" />
			<NumberField label="Wrong-password timeout (ms)" bind:value={cfg.fail_timeout_ms} min={0} max={10000} />
			<SaveButton onSave={save} />
		</Group>
	{/if}

	<Group title="Files">
		<button class="secondary" onclick={() => invoke("open_breadlock_config")}>Open breadlock.toml in editor</button>
		{#if examplePath}
			<button class="secondary" onclick={() => invoke("open_breadlock_example")}>Open example ({examplePath})</button>
		{:else}
			<Hint text="No packaged breadlock.example.toml found. The schema is [background] mode/path/blur, [clock] format, [font] family, [input] fail_timeout_ms." />
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
