<script lang="ts">
	let {
		label,
		value = $bindable(),
		options,
		emptyOptionsHint = "Nothing to pick from yet.",
	}: { label: string; value: string[]; options: string[]; emptyOptionsHint?: string } = $props();

	let adding = $state(false);

	let available = $derived(options.filter((o) => !value.includes(o)));

	function add(name: string) {
		if (!name) return;
		value = [...value, name];
		adding = false;
	}

	function remove(name: string) {
		value = value.filter((v) => v !== name);
	}
</script>

<div class="field">
	<div class="header">
		<span class="label">{label}</span>
	</div>
	<div class="chips">
		{#each value as name (name)}
			<span class="chip">
				{name}
				<button type="button" class="remove" onclick={() => remove(name)} aria-label={`Remove ${name}`}>×</button>
			</span>
		{/each}
		{#if adding}
			{#if available.length > 0}
				<!-- svelte-ignore a11y_autofocus -- responds to the user's own
				     "+" click just now, not a page-load focus grab -->
				<select autofocus onchange={(e) => add(e.currentTarget.value)}>
					<option value="" selected disabled>Choose…</option>
					{#each available as opt (opt)}
						<option value={opt}>{opt}</option>
					{/each}
				</select>
			{:else}
				<span class="empty-hint">{emptyOptionsHint}</span>
			{/if}
		{:else if available.length > 0}
			<button type="button" class="chip add" onclick={() => (adding = true)}>+</button>
		{/if}
	</div>
</div>

<style>
	.field {
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.label {
		display: block;
		margin-bottom: var(--space-xs, 4px);
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}

	.chip {
		display: flex;
		align-items: center;
		gap: 6px;
		background-color: var(--overlay);
		color: var(--on-overlay);
		border-radius: 999px;
		padding: 4px 6px 4px 12px;
		font-size: var(--font-size-secondary, 12px);
	}

	.chip.add {
		border: none;
		cursor: pointer;
		padding: 4px 10px;
		font-weight: bold;
		color: var(--on-surface);
	}

	.chip.add:hover {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	.remove {
		background: transparent;
		border: none;
		color: inherit;
		opacity: 0.6;
		cursor: pointer;
		font-size: 14px;
		line-height: 1;
		padding: 0;
	}

	.remove:hover {
		opacity: 1;
	}

	select {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid var(--accent);
		border-radius: var(--radius-secondary, 6px);
		padding: 4px var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
	}

	.empty-hint {
		opacity: 0.5;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
