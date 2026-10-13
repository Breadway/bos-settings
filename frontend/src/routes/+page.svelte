<script lang="ts">
	import { onMount, setContext } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import ChangeToast from "$lib/components/ChangeToast.svelte";
	import SearchPalette from "$lib/components/SearchPalette.svelte";
	import DraftNotice from "$lib/components/DraftNotice.svelte";
	import { drafts } from "$lib/drafts.svelte";
	import Sidebar from "$lib/components/Sidebar.svelte";
	import Placeholder from "$lib/components/Placeholder.svelte";
	import SectionError from "$lib/components/SectionError.svelte";
	import { go, nav, NAVIGATE_KEY, type Navigate } from "$lib/nav.svelte";
	import { initTheme } from "$lib/theme";
	import { VIEWS } from "$lib/views/registry";
	import "$lib/styles/app.css";

	let searchOpen = $state(false);
	let contentEl: HTMLElement;
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
				searchOpen = !searchOpen;
			}
		};
		const onUnload = (e: BeforeUnloadEvent) => { if (drafts.items.some(d => d.dirty)) { e.preventDefault(); e.returnValue = ""; } };
		window.addEventListener("beforeunload", onUnload);
		window.addEventListener("keydown", onKey);
		return () => {
			unlistenScreenshot.then((f) => f());
			unlistenNavigate.then((f) => f());
			window.removeEventListener("keydown", onKey);
			window.removeEventListener("beforeunload", onUnload);
		};
	});

 // A new page starts at the top. The content pane is shared across pages, so
 // without this the next page opened at the previous page's scroll offset.
 // Search jumps (focusLabel) and hub tab requests scroll on their own after.
 $effect(() => {
  void nav.page;
  if (contentEl) contentEl.scrollTop = 0;
 });

 $effect(() => {
  const target = nav.focusLabel;
  if (!target || !contentEl) return;
  function focusSetting() {
   const element = [...contentEl.querySelectorAll<HTMLElement>("[data-setting]")].find(el => el.dataset.setting === target);
   if (!element) return false;
   let ancestor = element.parentElement;
   while (ancestor && ancestor !== contentEl) { if (ancestor instanceof HTMLDetailsElement) ancestor.open = true; ancestor = ancestor.parentElement; }
   element.scrollIntoView({ block: "center" }); element.tabIndex = -1; element.focus({ preventScroll: true });
   element.classList.add("search-highlight"); setTimeout(() => element.classList.remove("search-highlight"), 2200);
   nav.focusLabel = ""; return true;
  }
  const observer = new MutationObserver(() => { if (focusSetting()) observer.disconnect(); });
  observer.observe(contentEl, { childList: true, subtree: true });
  focusSetting(); const timeout = setTimeout(() => observer.disconnect(), 15000);
  return () => { observer.disconnect(); clearTimeout(timeout); };
 });
</script>

<div class="app">
	<div class="shell">
		<Sidebar openSearch={() => searchOpen = true} />
		<main class="content" bind:this={contentEl} inert={drafts.saving}>
			{#key nav.page}
				{#if ActiveView}
					<svelte:boundary onerror={(e) => console.error("page failed", e)}>
						<ActiveView />
						{#snippet failed(error, reset)}
							<div class="page-error"><SectionError {error} {reset} /></div>
						{/snippet}
					</svelte:boundary>
				{:else}
					<Placeholder page={nav.page} />
				{/if}
			{/key}
		</main>
	</div>
	<SearchPalette bind:open={searchOpen} />
	<DraftNotice />
	<ChangeToast />
</div>

<style>
	.page-error {
		max-width: 760px;
		margin: 28px auto;
		padding: 0 24px;
	}
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
	@media (max-width: 560px) { .shell { flex-direction: column; } }
</style>
