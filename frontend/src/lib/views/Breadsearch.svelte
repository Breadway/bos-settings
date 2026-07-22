<script lang="ts">
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
	import SaveButton from "$lib/components/SaveButton.svelte";

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
</script>

<ViewScaffold title="File Search">
	<ServiceControl unit="breadmill.service" hasConfig />

	{#if cfg}
		<Group
			title="Power"
			hint="breadmill's embedding step is CPU/NPU/GPU-heavy. Turn it off entirely, or just pause it on battery — it resumes automatically on AC power."
		>
			<SwitchField label="Enabled" bind:value={cfg.power_enabled} />
			<SwitchField label="Index while on battery" bind:value={cfg.run_on_battery} />
		</Group>

		<Group
			title="Model"
			hint="What hardware does the indexing. Check journalctl --user -u breadmill after restarting to confirm it registered."
		>
			<Row label="Search acceleration">
				<select bind:value={cfg.backend}>
					{#each Object.entries(BACKEND_LABELS) as [code, label] (code)}
						<option value={code}>{label}</option>
					{/each}
				</select>
			</Row>
			<NumberField label="Max file size (MB)" bind:value={cfg.max_file_mb} min={0.1} max={500} step={0.5} />
		</Group>

		<Group title="Search">
			<NumberField label="Result limit" bind:value={cfg.search_limit} min={1} max={100} />
			<NumberField label="Snippet length" bind:value={cfg.snippet_len} min={20} max={2000} step={20} />
		</Group>

		<Group title="Index" wide>
			<PathListField label="Folders to index" bind:value={cfg.index_roots} />
			<PathListField label="Excluded folders" bind:value={cfg.index_excludes} hint="Skipped even if inside an indexed folder above." />
			<TagsField label="File extensions" bind:value={cfg.index_extensions} />
		</Group>

		<SaveButton onSave={save} />
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
