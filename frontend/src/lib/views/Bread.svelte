<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import TagsField from "$lib/components/TagsField.svelte";
	import ChipPickerField from "$lib/components/ChipPickerField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	interface BreadConfig {
		log_level: string;
		socket_path: string;
		lua_entry_point: string;
		lua_module_path: string;
		modules_builtin: boolean;
		modules_disable: string[];
		adapter_hyprland: boolean;
		adapter_udev: boolean;
		udev_subsystems: string[];
		adapter_power: boolean;
		power_poll_interval_secs: number;
		adapter_network: boolean;
		adapter_bluetooth: boolean;
		dedup_window_ms: number;
		notif_default_timeout_ms: number;
		notif_default_urgency: string;
		notif_notify_send_path: string;
	}

	let cfg = $state<BreadConfig | null>(null);
	// The daemon's 4 compiled-in modules plus every *.lua file actually
	// sitting in the configured module directory  -  real installed modules,
	// not a guess, so picking one to disable is a click.
	let knownModules = $state<string[]>([]);

	onMount(async () => {
		cfg = await invoke<BreadConfig>("get_bread_config");
		knownModules = await invoke<string[]>("list_bread_modules");
	});

	async function save() {
		await invoke("save_bread_config", { cfg });
	}
</script>

<ViewScaffold title="Daemon">
	<ServiceControl unit="breadd.service" critical hasConfig />

	{#if cfg}
		<Group title="Daemon">
			<SelectField label="Log level" bind:value={cfg.log_level} options={["error", "warn", "info", "debug", "trace"]} />
			<TextField label="Socket path" bind:value={cfg.socket_path} placeholder="default (XDG runtime dir)" />
		</Group>

		<Group title="Lua">
			<TextField label="Entry point" bind:value={cfg.lua_entry_point} placeholder="~/.config/bread/init.lua" />
			<TextField label="Module path" bind:value={cfg.lua_module_path} placeholder="~/.config/bread/modules" />
		</Group>

		<Group title="Modules">
			<SwitchField label="Load built-in modules" bind:value={cfg.modules_builtin} />
			<ChipPickerField
				label="Disabled modules"
				bind:value={cfg.modules_disable}
				options={knownModules}
				emptyOptionsHint="No other modules found to disable."
			/>
		</Group>

		<Group title="Adapters" hint="Sources breadd normalises into events. Disable any you don't use.">
			<SwitchField label="Hyprland" bind:value={cfg.adapter_hyprland} />
			<SwitchField label="udev (devices)" bind:value={cfg.adapter_udev} />
			<TagsField label="udev subsystems" bind:value={cfg.udev_subsystems} />
			<SwitchField label="Power" bind:value={cfg.adapter_power} />
			<NumberField label="Power poll interval (s)" bind:value={cfg.power_poll_interval_secs} min={1} max={3600} />
			<SwitchField label="Network" bind:value={cfg.adapter_network} />
			<SwitchField label="Bluetooth" bind:value={cfg.adapter_bluetooth} />
		</Group>

		<Group title="Events">
			<NumberField label="Dedup window (ms)" bind:value={cfg.dedup_window_ms} min={0} max={10000} step={50} />
		</Group>

		<Group title="Notifications">
			<NumberField label="Default timeout (ms)" bind:value={cfg.notif_default_timeout_ms} min={0} max={60000} step={500} />
			<SelectField label="Default urgency" bind:value={cfg.notif_default_urgency} options={["low", "normal", "critical"]} />
			<TextField label="notify-send path" bind:value={cfg.notif_notify_send_path} placeholder="auto-detected" />
		</Group>

		<SaveButton onSave={save} />
	{/if}
</ViewScaffold>
