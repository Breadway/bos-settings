<script lang="ts">
	import { drafts, saveDrafts, discardAndContinue } from "$lib/drafts.svelte";
	let dialog: HTMLDialogElement;
	// Settings save themselves, so the dialog only appears when a save failed
	// while leaving the page: retry, discard, or stay and fix it.
	let blocked = $derived(drafts.pending !== null && drafts.error !== "" && !drafts.saving);
	$effect(() => { if (blocked && dialog && !dialog.open) dialog.showModal(); else if (!blocked) dialog?.close(); });
</script>
{#if drafts.error && !blocked}
	<div class="notice error-notice" role="alert"><span>Couldn't save: {drafts.error}</span><button class="btn" disabled={drafts.saving} onclick={saveDrafts}>Try again</button></div>
{:else if drafts.status}
	<div class="notice saved" role="status">{drafts.status}</div>
{/if}
<dialog bind:this={dialog} aria-labelledby="draft-title" oncancel={(e) => { e.preventDefault(); drafts.pending = null; }}>
	<h2 id="draft-title">Your last change wasn't saved</h2>
	<p class="error" role="alert">{drafts.error}</p>
	<div class="actions"><button class="btn" onclick={() => drafts.pending = null}>Stay here</button><button class="btn danger" onclick={discardAndContinue}>Leave without it</button><button class="btn primary" onclick={saveDrafts}>Try again</button></div>
</dialog>
<style>
	.notice { position:fixed; bottom:16px; left:50%; transform:translateX(-50%); max-width:calc(100% - 32px); z-index:20; display:flex; align-items:center; gap:14px; flex-wrap:wrap; padding:10px 14px; background:var(--surface-2); border:1px solid var(--line); border-radius:12px; box-shadow:0 10px 40px #0006; }
	.saved { padding:7px 14px; color:var(--muted); font-size:12.5px; pointer-events:none; }
	.error-notice span { color:var(--red); }
	dialog { max-width:460px; width:calc(100% - 32px); color:var(--fg); background:var(--surface); border:1px solid var(--line); border-radius:16px; padding:20px; } dialog::backdrop { background:#04050799; }
	h2 { font-size:18px; font-weight:500; margin:0; } p { color:var(--muted); } .actions { display:flex; gap:8px; flex-wrap:wrap; justify-content:flex-end; } .error { color:var(--red); }
</style>
