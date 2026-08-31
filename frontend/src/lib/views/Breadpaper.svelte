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
		scanning = true;
		try {
			library = await invoke<LibraryEntry[]>("list_wallpaper_library");
		} finally {
			scanning = false;
		}
	});

	async function apply(path: string) {
		status = "Setting…";
		try {
			await invoke("set_wallpaper", { path });
			await refreshCurrent();
			status = "Set";
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
		if (typeof path === "string") await apply(path);
	}
</script>

<ViewScaffold title="Wallpaper">
	<Group title="Wallpaper" hint="This also updates desktop colors.">
		{#if currentPath}
			<div class="hero" style="background-image: url('{convertFileSrc(currentPath)}')">
				<span class="cap">{currentPath.split("/").pop()}</span>
			</div>
		{:else}
			<div class="hero placeholder">No wallpaper</div>
		{/if}
		<div class="btn-row">
			<button class="btn primary" onclick={chooseImage}>Choose image</button>
			<span class="status">{status}</span>
		</div>
	</Group>

	<Group title="Library" wide>
		{#if scanning && library === null}
			<div class="empty">Loading {libraryDir}…</div>
		{:else if library && library.length === 0}
			<div class="empty">Nothing in {libraryDir}</div>
		{:else if library}
			<div class="grid">
				{#each library as item (item.path)}
					<button
						class="thumb"
						class:on={item.path === currentPath}
						onclick={() => apply(item.path)}
						title={item.path}
					>
						<img src={convertFileSrc(item.path)} alt={item.name} loading="lazy" />
					</button>
				{/each}
			</div>
		{/if}
	</Group>
</ViewScaffold>

<style>
	.hero {
		height: 180px;
		border-radius: 12px;
		background-size: cover;
		background-position: center;
		position: relative;
		overflow: hidden;
	}

	.hero.placeholder {
		display: grid;
		place-items: center;
		background: var(--bg);
		color: var(--muted);
	}

	.cap {
		position: absolute;
		left: 12px;
		bottom: 10px;
		font-size: 12px;
		color: #fffc;
		background: #0006;
		padding: 4px 8px;
		border-radius: 6px;
	}

	.btn-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 12px;
	}

	.status {
		color: var(--muted);
		font-size: 12px;
	}

	.empty {
		color: var(--muted);
		font-size: 12px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
		gap: 8px;
	}

	.thumb {
		padding: 0;
		border: 2px solid transparent;
		border-radius: 8px;
		overflow: hidden;
		cursor: pointer;
		background: transparent;
		aspect-ratio: 16/10;
	}

	.thumb.on {
		border-color: var(--accent);
	}

	.thumb img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
