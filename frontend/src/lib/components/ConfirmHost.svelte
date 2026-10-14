<script lang="ts">
	import Sheet from "./Sheet.svelte";
	import { confirmState, answer } from "$lib/confirm.svelte";
</script>

{#if confirmState.pending}
	{@const req = confirmState.pending}
	<Sheet title={req.title} onClose={() => answer(false)}>
		{#if req.message}<p class="msg">{req.message}</p>{/if}
		{#snippet actions()}
			<button class="btn" onclick={() => answer(false)}>{req.cancelLabel ?? "Cancel"}</button>
			<button class="btn {req.danger === false ? 'primary' : 'danger'}" onclick={() => answer(true)}>{req.confirmLabel ?? "Continue"}</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.msg { margin:0; color:var(--muted); font-size:13px; line-height:1.45; white-space:pre-line; overflow-wrap:anywhere; }
</style>
