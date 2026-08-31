<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	interface ShotBind {
		shortcut: string;
		command: string;
	}
	interface BreadshotConfig {
		save_dir: string;
		silent: boolean;
		freeze: boolean;
		notif_timeout: number;
		date_format: string;
	}

	let binds = $state<ShotBind[] | null>(null);
	let cfg = $state<BreadshotConfig | null>(null);

	onMount(async () => {
		binds = await invoke<ShotBind[]>("get_breadshot_binds");
		cfg = await invoke<BreadshotConfig>("get_breadshot_config");
	});

	async function save() {
		await invoke("save_breadshot_config", { cfg });
	}
</script>

<ViewScaffold title="Screenshots">
	<Group
		title="Keybinds"
		hint="Change shortcuts under Keyboard."
	>
		{#if binds && binds.length > 0}
			{#each binds as b (`${b.shortcut}:${b.command}`)}
				<InfoRow label={b.shortcut} value={b.command} />
			{/each}
		{:else}
			<Hint text="No breadshot binds found." />
		{/if}
		<button class="primary" onclick={() => invoke("breadshot_region_clipboard")}>Capture region to clipboard</button>
	</Group>

	{#if cfg}
		<Group title="Capture">
			<FileField label="Save directory" bind:value={cfg.save_dir} placeholder="~/Pictures/Screenshots" mode="folder" />
			<SwitchField label="Silent (no notifications)" bind:value={cfg.silent} />
			<SwitchField label="Freeze screen during select" bind:value={cfg.freeze} />
			<Hint text="Freeze needs hyprpicker. Filenames are <date_format>_breadshot.png." />
			<NumberField label="Notification timeout (ms)" bind:value={cfg.notif_timeout} min={0} max={60000} />
			<TextField label="Filename date format" bind:value={cfg.date_format} placeholder="%Y-%m-%d-%H%M%S" />
			<SaveButton onSave={save} />
		</Group>
	{/if}
</ViewScaffold>

<style>
	.primary {
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}
</style>
