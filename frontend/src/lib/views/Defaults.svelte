<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import Row from "$lib/components/Row.svelte";

	interface DesktopApp {
		id: string;
		name: string;
	}
	interface DefaultsStatus {
		path: string;
		current: Record<string, string>;
		options: Record<string, DesktopApp[]>;
	}

	const CATEGORIES: { id: string; label: string }[] = [
		{ id: "browser", label: "Browser" },
		{ id: "files", label: "File manager" },
		{ id: "terminal", label: "Terminal" },
		{ id: "image", label: "Images" },
		{ id: "pdf", label: "PDF" },
		{ id: "editor", label: "Editor" },
	];

	let st = $state<DefaultsStatus | null>(null);

	onMount(async () => {
		st = await invoke<DefaultsStatus>("get_default_apps");
	});

	async function save() {
		if (!st) return;
		await invoke("save_default_apps", { input: { current: st.current } });
	}
</script>

<ViewScaffold title="Default apps">
	<Group title="Apps" wide>
		{#if st}
			{#each CATEGORIES as cat (cat.id)}
				<Row label={cat.label}>
					<select bind:value={st.current[cat.id]} onchange={save}>
						<option value="">Not set</option>
						{#each st.options[cat.id] ?? [] as app (app.id)}
							<option value={app.id}>{app.name}</option>
						{/each}
					</select>
				</Row>
			{/each}
			<Hint text="Applies as you change it." />
		{:else}
			<Hint text="Loading…" />
		{/if}
	</Group>
</ViewScaffold>

<style>
	select {
		color-scheme: dark;
		background: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: 10px;
		padding: 6px 10px;
		min-width: 22ch;
		max-width: 36ch;
	}

	select:focus {
		border-color: var(--accent);
	}
</style>
