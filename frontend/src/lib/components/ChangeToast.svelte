<script lang="ts">
 import { changes } from "$lib/changes.svelte";
 // Fades on its own: long enough to reach Undo, short enough not to linger.
 $effect(() => {
  const message = changes.message;
  if (!message) return;
  const t = setTimeout(() => { if (changes.message === message && !changes.busy) { changes.message = ""; changes.undo = null; } }, changes.undo ? 8000 : 3500);
  return () => clearTimeout(t);
 });
</script>
{#if changes.message}
 <div class="toast" role="status"><span>{changes.message}</span>{#if changes.undo}<button class="btn" disabled={changes.busy} onclick={() => changes.undo?.()}>Undo</button>{/if}<button class="dismiss" aria-label="Dismiss notification" onclick={() => {changes.message = ""; changes.undo = null;}}>×</button></div>
{/if}
<style>.toast { position:fixed; bottom:80px; left:50%; transform:translateX(-50%); max-width:calc(100% - 32px); z-index:25; display:flex; align-items:center; gap:12px; padding:10px 14px; border:1px solid var(--line); border-radius:12px; background:var(--surface-2); box-shadow:0 10px 40px #0006; } .dismiss { border:0; background:none; color:var(--muted); padding:4px; cursor:pointer; font-size:20px; }</style>
