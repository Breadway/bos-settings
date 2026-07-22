<script lang="ts">
	import { onMount } from "svelte";
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";

	interface LibraryEntry {
		path: string;
		name: string;
	}

	let currentPath = $state<string | null>(null);
	let status = $state("");
	let libraryDir = $state("");
	let library = $state<LibraryEntry[] | null>(null);
	let scanning = $state(false);

	async function refreshCurrent() {
		currentPath = await invoke<string | null>("get_current_wallpaper");
	}

	onMount(async () => {
		libraryDir = await invoke<string>("wallpaper_library_dir_display");
		await refreshCurrent();
	});

	async function apply(path: string) {
		status = "Setting…";
		try {
			await invoke("set_wallpaper", { path });
			await refreshCurrent();
			status = "Wallpaper set";
		} catch (e) {
			status = `${e}`;
		} finally {
			setTimeout(() => (status = ""), 3000);
		}
	}

	async function chooseImage() {
		const path = await open({
			title: "Choose a wallpaper",
			filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }],
		});
		if (typeof path === "string") {
			await apply(path);
		}
	}

	async function browseLibrary() {
		scanning = true;
		library = await invoke<LibraryEntry[]>("list_wallpaper_library");
		scanning = false;
	}
</script>

<ViewScaffold title="Wallpaper">
	<Group
		title="Current wallpaper"
		hint="Sets the desktop wallpaper, generates a matching pywal palette, and reloads the shared bread-theme stylesheet — the wallpaper drives the whole desktop's accent colors."
	>
		<div class="preview-card">
			{#if currentPath}
				<img class="preview" src={convertFileSrc(currentPath)} alt="Current wallpaper" />
				<span class="path">{currentPath.split("/").pop()}</span>
			{:else}
				<div class="preview placeholder">No wallpaper set</div>
			{/if}
		</div>

		<div class="btn-row">
			<button class="choose" onclick={chooseImage}>Choose image…</button>
			<span class="status">{status}</span>
		</div>
	</Group>

	<Group title="Library" wide>
		{#if library === null}
			<button class="browse" disabled={scanning} onclick={browseLibrary}>
				{scanning ? "Scanning…" : `Browse ${libraryDir}`}
			</button>
		{:else if library.length === 0}
			<div class="empty">Nothing under {libraryDir} — use Choose image… above instead.</div>
		{:else}
			<div class="grid">
				{#each library as item (item.path)}
					<button class="thumb" onclick={() => apply(item.path)} title={item.path}>
						<img src={convertFileSrc(item.path)} alt={item.name} loading="lazy" />
						<span class="name">{item.name}</span>
					</button>
				{/each}
			</div>
		{/if}
	</Group>
</ViewScaffold>

<style>
	.preview-card {
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-sm, 8px);
	}

	.preview {
		display: block;
		width: 320px;
		height: 180px;
		object-fit: cover;
		border-radius: var(--radius-secondary, 6px);
	}

	.preview.placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		opacity: 0.5;
	}

	.path {
		display: block;
		margin-top: var(--space-xs, 4px);
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.btn-row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		justify-content: center;
		margin-top: var(--space-sm, 8px);
	}

	.choose,
	.browse {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}

	.browse {
		background-color: var(--surface);
		color: var(--on-surface);
	}

	.browse:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.empty {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
		gap: var(--space-sm, 8px);
	}

	.thumb {
		background: transparent;
		border: none;
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 0;
	}

	.thumb img {
		display: block;
		width: 100%;
		height: 88px;
		object-fit: cover;
		border-radius: var(--radius-tertiary, 4px);
	}

	.thumb .name {
		font-size: 11px;
		opacity: 0.6;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
