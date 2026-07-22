<script lang="ts">
	import { onMount } from "svelte";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import Placeholder from "$lib/components/Placeholder.svelte";
	import { DEFAULT_PAGE } from "$lib/sidebar";
	import { initTheme } from "$lib/theme";
	import { VIEWS } from "$lib/views/registry";

	let activePage = $state(DEFAULT_PAGE);
	let ActiveView = $derived(VIEWS[activePage]);

	onMount(() => {
		initTheme();
	});
</script>

<div class="shell">
	<Sidebar bind:activePage />
	<main class="content">
		{#if ActiveView}
			<ActiveView />
		{:else}
			<Placeholder page={activePage} />
		{/if}
	</main>
</div>

<style>
	:global(*) {
		box-sizing: border-box;
	}

	:global(html, body) {
		margin: 0;
		border: none;
		outline: none;
		height: 100%;
		color-scheme: dark;
		background-color: var(--bg, #0c0c0c);
		color: var(--fg);
		font-family: var(--font-family, sans-serif);
		font-size: var(--font-size-base, 14px);
	}

	:global(#svelte) {
		height: 100%;
	}

	.shell {
		display: flex;
		height: 100vh;
		background-color: var(--bg, #0c0c0c);
	}

	.content {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		overscroll-behavior: contain;
	}
</style>
