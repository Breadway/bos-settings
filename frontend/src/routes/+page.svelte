<script lang="ts">
	import { onMount, setContext } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import Placeholder from "$lib/components/Placeholder.svelte";
	import { go, nav, NAVIGATE_KEY, type Navigate } from "$lib/nav.svelte";
	import { initTheme } from "$lib/theme";
	import { VIEWS } from "$lib/views/registry";
	import "$lib/styles/app.css";

	let ActiveView = $derived(VIEWS[nav.page]);

	setContext<Navigate>(NAVIGATE_KEY, go);

	onMount(() => {
		initTheme();
		// `--page <id>` on this launch, if any: read once, before first
		// paint decides which view to render, so there's no flash of Home.
		// A command rather than an event, since an event emitted from Rust's
		// `setup()` would race this listener not being attached yet.
		invoke<string | null>("get_initial_page").then((initialPage) => {
			if (initialPage) go(initialPage);
		});
		const unlistenScreenshot = listen<string>("screenshot-set-view", (event) => {
			go(event.payload);
		});
		// Fired by the single-instance plugin's second-instance callback: a
		// later `--page <id>` launch while this window is already up
		// re-navigates it instead of spawning a duplicate process.
		const unlistenNavigate = listen<string>("navigate-to-page", (event) => {
			go(event.payload);
		});
		const onKey = (e: KeyboardEvent) => {
			if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
				e.preventDefault();
				nav.searchNonce += 1;
				go("home");
			}
		};
		window.addEventListener("keydown", onKey);
		return () => {
			unlistenScreenshot.then((f) => f());
			unlistenNavigate.then((f) => f());
			window.removeEventListener("keydown", onKey);
		};
	});
</script>

<div class="app">
	<div class="shell">
		<Sidebar />
		<main class="content">
			{#key nav.page}
				{#if ActiveView}
					<ActiveView />
				{:else}
					<Placeholder page={nav.page} />
				{/if}
			{/key}
		</main>
	</div>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background-color: var(--bg, #12161c);
	}

	.shell {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	.content {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
</style>
