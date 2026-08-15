<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

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
		st = await invoke<DefaultsStatus>("get_default_apps");
	}
</script>

<ViewScaffold title="Default apps">
	<Group title="MIME associations" hint={st ? `Read and written at ${st.path}. Terminal also writes ~/.config/xdg-terminals.list.` : "Loading…"} wide>
		{#if st}
			{#each CATEGORIES as cat (cat.id)}
				<div class="field-row">
					<span class="label">{cat.label}</span>
					<select bind:value={st.current[cat.id]}>
						<option value="">—</option>
						{#each st.options[cat.id] ?? [] as app (app.id)}
							<option value={app.id}>{app.name}</option>
						{/each}
					</select>
				</div>
			{/each}
			<SaveButton onSave={save} />
		{:else}
			<Hint text="Loading…" />
		{/if}
	</Group>
</ViewScaffold>

<style>
	.field-row {
		display: flex;
		align-items: center;
		gap: var(--space-lg, 16px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	:global(.field-row + .field-row) {
		margin-top: calc(var(--space-sm, 8px) * -1);
		border-top: 1px solid var(--bg);
		border-top-left-radius: 0;
		border-top-right-radius: 0;
	}

	:global(.field-row:has(+ .field-row)) {
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
		margin-bottom: 0;
	}

	.label {
		flex: 1;
	}

	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
		max-width: 28ch;
	}

	select:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
