<script lang="ts">
	let { onSave }: { onSave: () => Promise<void> } = $props();

	let status = $state("");
	let saving = $state(false);

	async function save() {
		saving = true;
		try {
			await onSave();
			status = "Saved";
			setTimeout(() => (status = ""), 3000);
		} catch (e) {
			status = `Error: ${e}`;
		} finally {
			saving = false;
		}
	}
</script>

<div class="row">
	<button disabled={saving} onclick={save}>Save</button>
	<span class="status">{status}</span>
</div>

<style>
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		margin-top: var(--space-lg, 16px);
		/* Always spans the full grid width, regardless of how many
		   Group columns the rest of the page laid out above it. */
		grid-column: 1 / -1;
	}

	button {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
