<script lang="ts">
	import { onMount } from "svelte";
	import { invoke, convertFileSrc } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface LibraryEntry {
		path: string;
		name: string;
	}
	interface LiveMonitor {
		name: string;
		mode: string;
	}
	interface OutputWallpaper {
		output: string;
		path: string;
	}

	let monitors = $state<LiveMonitor[]>([]);
	let byOutput = $state<Record<string, string>>({});
	let globalPath = $state<string | null>(null);
	let selected = $state<string | null>(null);
	let sameOnAll = $state(true);

	let status = $state("");
	let libraryDir = $state("");
	let library = $state<LibraryEntry[] | null>(null);
	let scanning = $state(false);

	// The wallpaper the current selection resolves to: the global one when
	// "same on every monitor" is on, otherwise the selected monitor's own
	// (falling back to the global if that monitor was never set explicitly).
	let activePath = $derived(
		sameOnAll ? globalPath : (selected && byOutput[selected]) || globalPath,
	);

	const perMonitor = $derived(monitors.length > 1);

	async function refresh() {
		globalPath = await invoke<string | null>("get_current_wallpaper");
		const list = await invoke<OutputWallpaper[]>("get_wallpapers_by_output");
		byOutput = Object.fromEntries(list.map((o) => [o.output, o.path]));
		monitors = await invoke<LiveMonitor[]>("get_live_monitors");
		if (!selected || !monitors.some((m) => m.name === selected)) {
			selected = monitors[0]?.name ?? null;
		}
	}

	onMount(async () => {
		libraryDir = await invoke<string>("wallpaper_library_dir_display");
		await refresh();
		// Start in per-monitor mode only if the monitors genuinely disagree.
		const distinct = new Set(Object.values(byOutput));
		sameOnAll = distinct.size <= 1;
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
			if (sameOnAll || !selected) {
				await invoke("set_wallpaper", { path });
			} else {
				await invoke("set_wallpaper_on", { output: selected, path });
			}
			await refresh();
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

	function shortName(p: string | null): string {
		return p ? (p.split("/").pop() ?? p) : "";
	}
</script>

<ViewScaffold title="Wallpaper">
	<Group title="Wallpaper" hint="This also sets the desktop colors — each monitor's accent comes from its own wallpaper.">
		{#if perMonitor}
			<SwitchField label="Use the same wallpaper on every monitor" bind:value={sameOnAll} />
		{/if}

		{#if perMonitor && !sameOnAll}
			<div class="mons">
				{#each monitors as m (m.name)}
					<button
						type="button"
						class="mon"
						class:on={selected === m.name}
						title={m.mode}
						onclick={() => (selected = m.name)}
					>
						{m.name}
					</button>
				{/each}
			</div>
		{/if}

		{#if activePath}
			<div class="hero" style="background-image: url('{convertFileSrc(activePath)}')">
				<span class="cap">
					{#if perMonitor && !sameOnAll}{selected} · {/if}{shortName(activePath)}
				</span>
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
						class:on={item.path === activePath}
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
	.mons {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 4px 0 12px;
	}

	.mon {
		padding: 6px 11px;
		border-radius: 999px;
		font-size: 12px;
		background: color-mix(in srgb, var(--fg) 6%, transparent);
		border: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
		color: var(--fg);
		cursor: pointer;
	}

	.mon.on {
		background: var(--accent);
		color: var(--on-accent);
		border-color: transparent;
		font-weight: 600;
	}

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
