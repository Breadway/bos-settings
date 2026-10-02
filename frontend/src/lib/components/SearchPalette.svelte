<script lang="ts">
	import { tick } from "svelte";
	import Search from "@lucide/svelte/icons/search";
	import { searchSettings } from "$lib/search";
	import { go } from "$lib/nav.svelte";
	let { open = $bindable(false) }: { open?: boolean } = $props();
	let dialog: HTMLDialogElement;
	let input: HTMLInputElement;
	let query = $state("");
	let selected = $state(0);
	let hits = $derived(searchSettings(query, 12));
	$effect(() => { if (open && dialog) { query = ""; selected = 0; dialog.showModal(); tick().then(() => input?.focus()); } else dialog?.close(); });
	$effect(() => { void query; selected = 0; });
	function jump(index: number) {
		const hit = hits[index]; if (!hit) return;
		open = false; go(hit.page, hit.tab, hit.target);
	}
	function onKey(e: KeyboardEvent) {
		if (e.key === "ArrowDown" || e.key === "ArrowUp") {
			e.preventDefault(); selected = Math.max(0, Math.min(hits.length - 1, selected + (e.key === "ArrowDown" ? 1 : -1)));
			dialog.querySelector(`[data-index="${selected}"]`)?.scrollIntoView({ block: "nearest" });
		} else if (e.key === "Enter") { e.preventDefault(); jump(selected); }
	}
</script>

<dialog bind:this={dialog} class="palette" onclose={() => open = false} oncancel={() => open = false} aria-label="Search settings">
	<div class="search"><Search size={20} /><input bind:this={input} bind:value={query} onkeydown={onKey} aria-label="Find a setting" placeholder="Find a setting…" /><button class="btn" onclick={() => open = false}>Esc</button></div>
	<div class="results">
		{#each hits as hit, i (`${hit.page}:${hit.tab ?? ""}:${hit.label}`)}
			<button class="result" class:selected={selected === i} data-index={i} onclick={() => jump(i)}>
				<span><b>{hit.label}</b><small>{hit.context ?? hit.page}</small></span><span aria-hidden="true">↵</span>
			</button>
		{:else}
			<p class="empty">No settings found for “{query}”. Try another name or keyword.</p>
		{/each}
	</div>
	<footer>↑ ↓ to choose · Enter to open · Esc to close</footer>
</dialog>

<style>
	.palette { position:fixed; top:min(70px,10vh); margin:0 auto; width:min(600px,calc(100% - 32px)); max-height:80vh; padding:0; border:1px solid var(--line); border-radius:16px; background:var(--surface); color:var(--fg); box-shadow:0 30px 80px #0008; }
	.palette::backdrop { background:#04050799; backdrop-filter:blur(4px); }
	.search { display:flex; align-items:center; gap:10px; padding:14px 16px; border-bottom:1px solid var(--line); }
	input { flex:1; min-width:0; background:none; border:0; font-size:17px; }
	.results { max-height:380px; overflow-y:auto; padding:6px; }
	.result { display:flex; align-items:center; justify-content:space-between; gap:12px; width:100%; padding:9px 10px; border:0; background:none; color:inherit; text-align:left; border-radius:10px; cursor:pointer; }
	.result.selected,.result:hover { background:var(--accent-soft); }
	.result b { display:block; font-weight:400; } .result small { display:block; color:var(--muted); font-size:11.5px; margin-top:2px; }
	footer,.empty { padding:12px 16px; font-size:12px; color:var(--muted); } footer { border-top:1px solid var(--line); }
</style>
