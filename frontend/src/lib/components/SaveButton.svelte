<script lang="ts">
	import { drafts, markActionSaved } from "$lib/drafts.svelte";
	let { onSave }: { onSave: () => Promise<void> } = $props();

	let status = $state("");
	let saving = $state(false);

	async function save() {
		saving = true;
		drafts.saving = true;
		try {
			await onSave();
			markActionSaved(onSave);
			status = "Saved";
			setTimeout(() => (status = ""), 3000);
		} catch (e) {
			status = `Error: ${e}`;
		} finally {
			saving = false;
			drafts.saving = false;
		}
	}
</script>

<div class="row">
	<button class="btn primary" disabled={saving || drafts.saving} onclick={save}>Save</button>
	<span class="status">{status}</span>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		margin-top: 4px;
	}

	.status {
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		font-size: var(--font-size-secondary, 12px);
	}
</style>
