<script lang="ts">
	import { drafts, saveDrafts, discardAndContinue } from "$lib/drafts.svelte";
	let dialog: HTMLDialogElement;
	let dirty = $derived(drafts.items.some(d => d.dirty));
	$effect(() => { if (drafts.pending && dialog && !dialog.open) dialog.showModal(); else if (!drafts.pending) dialog?.close(); });
</script>
{#if dirty}
	<div class="notice" role="status"><span>Unsaved changes</span><button class="btn primary" disabled={drafts.saving} onclick={saveDrafts}>{drafts.saving ? "Saving…" : "Save changes"}</button>{#if drafts.error}<span class="error">{drafts.error}</span>{/if}</div>
{/if}
<dialog bind:this={dialog} aria-labelledby="draft-title" oncancel={(e) => { e.preventDefault(); if (!drafts.saving) drafts.pending = null; }}>
	<h2 id="draft-title">Save your changes?</h2><p>Your edits have not been saved yet.</p>
	{#if drafts.error}<p class="error" role="alert">{drafts.error}</p>{/if}
	<div class="actions"><button class="btn" disabled={drafts.saving} onclick={() => drafts.pending = null}>Keep editing</button><button class="btn danger" disabled={drafts.saving} onclick={discardAndContinue}>Discard</button><button class="btn primary" disabled={drafts.saving} onclick={saveDrafts}>Save and continue</button></div>
</dialog>
<style>
	.notice { position:fixed; bottom:16px; left:50%; transform:translateX(-50%); max-width:calc(100% - 32px); z-index:20; display:flex; align-items:center; gap:14px; flex-wrap:wrap; padding:10px 14px; background:var(--surface-2); border:1px solid var(--line); border-radius:12px; box-shadow:0 10px 40px #0006; }
	dialog { max-width:460px; width:calc(100% - 32px); color:var(--fg); background:var(--surface); border:1px solid var(--line); border-radius:16px; padding:20px; } dialog::backdrop { background:#04050799; }
	h2 { font-size:18px; font-weight:500; margin:0; } p { color:var(--muted); } .actions { display:flex; gap:8px; flex-wrap:wrap; justify-content:flex-end; } .error { color:var(--red); }
</style>
