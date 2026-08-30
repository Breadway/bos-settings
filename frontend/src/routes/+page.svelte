<script lang="ts">
	import { onMount, setContext } from "svelte";
	import { listen } from "@tauri-apps/api/event";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import Titlebar from "$lib/components/Titlebar.svelte";
	import Placeholder from "$lib/components/Placeholder.svelte";
	import { go, nav, NAVIGATE_KEY, type Navigate } from "$lib/nav.svelte";
	import { initTheme } from "$lib/theme";
	import { VIEWS } from "$lib/views/registry";
	import "$lib/styles/app.css";

	let ActiveView = $derived(VIEWS[nav.page]);

	setContext<Navigate>(NAVIGATE_KEY, go);

	onMount(() => {
		initTheme();
		const unlisten = listen<string>("screenshot-set-view", (event) => {
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
			unlisten.then((f) => f());
			window.removeEventListener("keydown", onKey);
		};
	});
</script>

<div class="app">
	<Titlebar />
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
