<script lang="ts">
	import type { Snippet } from "svelte";
	import { onMount } from "svelte";

	// A centred modal sheet, the proposal's editor and "Add a device" pattern.
	// Escape or a click on the dimmed backdrop closes it.
	let {
		title,
		lede,
		onClose,
		children,
		actions,
	}: { title: string; lede?: string; onClose: () => void; children: Snippet; actions?: Snippet } = $props();

	let sheet: HTMLDivElement;
	onMount(() => {
		const previous = document.activeElement as HTMLElement | null;
		sheet.querySelector<HTMLElement>("input, select, button")?.focus();
		const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") { e.stopPropagation(); onClose(); } };
		window.addEventListener("keydown", onKey, true);
		return () => { window.removeEventListener("keydown", onKey, true); previous?.focus?.(); };
	});
</script>

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
	<div class="sheet" role="dialog" aria-modal="true" aria-label={title} bind:this={sheet}>
		<h3>{title}</h3>
		{#if lede}<p class="lede">{lede}</p>{/if}
		<div class="body">{@render children()}</div>
		<div class="acts">
			{#if actions}{@render actions()}{:else}<button class="btn primary" onclick={onClose}>Done</button>{/if}
		</div>
	</div>
</div>

<style>
	.scrim { position:fixed; inset:0; z-index:150; display:grid; place-items:center; padding:20px; background:color-mix(in srgb, var(--bg) 62%, transparent); backdrop-filter:blur(3px); }
	.sheet { width:min(520px, 100%); max-height:calc(100vh - 40px); overflow-y:auto; background:var(--surface); border:1px solid var(--line); border-radius:16px; padding:20px; display:flex; flex-direction:column; gap:12px; box-shadow:0 30px 80px #0009; }
	h3 { margin:0; font:500 18px/1.2 var(--font-display, 'Outfit', 'Varela Round', sans-serif); }
	.lede { margin:0; color:var(--muted); font-size:13px; line-height:1.45; }
	.body { display:flex; flex-direction:column; min-width:0; }
	.acts { display:flex; gap:8px; justify-content:flex-end; flex-wrap:wrap; align-items:center; }
</style>
