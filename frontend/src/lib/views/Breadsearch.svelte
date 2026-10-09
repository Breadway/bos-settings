<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import Row from "$lib/components/Row.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import PathListField from "$lib/components/PathListField.svelte";
	import TagsField from "$lib/components/TagsField.svelte";

	const BACKEND_LABELS: Record<string, string> = {
		cpu: "CPU",
		npu: "NPU (AMD Ryzen AI)",
		rocm: "AMD GPU (ROCm)",
		cuda: "NVIDIA GPU (CUDA)",
		openvino: "Intel GPU (OpenVINO)",
	};

	interface BreadsearchConfig {
		power_enabled: boolean;
		run_on_battery: boolean;
		backend: string;
		index_roots: string[];
		index_excludes: string[];
		index_extensions: string[];
		max_file_mb: number;
		search_limit: number;
		snippet_len: number;
	}

	let cfg = $state<BreadsearchConfig | null>(null);

	onMount(async () => {
		cfg = await invoke<BreadsearchConfig>("get_breadsearch_config");
	});

	async function save() {
		await invoke("save_breadsearch_config", { cfg });
	}
	useDraft(() => (cfg), save, () => cfg !== null);
</script>

<ViewScaffold title="File search">
	{#if cfg}
		<Group title="File search" hint="Lets the launcher find files by what's inside them, not only by name.">
			<SwitchField label="Add new and changed files" hint={cfg.power_enabled ? "Reads files in the folders below so their contents can be found." : "Paused. Search still finds what was already read."} bind:value={cfg.power_enabled} />
			{#if cfg.power_enabled}
				<SwitchField label="Keep going on battery" hint="Reading files uses a lot of power, so it waits for the charger by default." bind:value={cfg.run_on_battery} />
			{/if}
			<PathListField label="Folders to search" bind:value={cfg.index_roots} />
			<details class="ui-disclosure">
				<summary>More</summary>
				<PathListField label="Skip these folders" bind:value={cfg.index_excludes} hint="Even when they are inside a folder above." />
				<TagsField label="File types" bind:value={cfg.index_extensions} />
				<Row label="Speed up with" hint="Which chip does the indexing work.">
					<select bind:value={cfg.backend}>
						{#each Object.entries(BACKEND_LABELS) as [code, label] (code)}
							<option value={code}>{label}</option>
						{/each}
					</select>
				</Row>
				<NumberField label="Skip files bigger than (MB)" bind:value={cfg.max_file_mb} min={0.1} max={500} step={0.5} />
				<NumberField label="Results to show" bind:value={cfg.search_limit} min={1} max={100} />
				<NumberField label="Preview length (characters)" bind:value={cfg.snippet_len} min={20} max={2000} step={20} />
				<ServiceControl unit="breadmill.service" name="Search indexer" bare hasConfig />
			</details>
		</Group>
	{/if}
</ViewScaffold>

<style>
	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	option {
		background-color: var(--bg);
		color: var(--on-surface);
	}

	select:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
