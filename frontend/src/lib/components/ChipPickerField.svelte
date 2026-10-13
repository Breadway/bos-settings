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

<div class="field" data-setting={label}>
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
			<button type="button" class="chip add" onclick={() => (adding = true)}>+ Add</button>
		{/if}
	</div>
</div>

<style>
	.field { padding: 10px 0; }
	.label { display: block; margin-bottom: 8px; font-size: 13px; }
	.chips { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
	.chip { display: flex; align-items: center; gap: 6px; background: var(--accent-soft); color: var(--fg); border: 1px solid color-mix(in oklab, var(--accent) 45%, transparent); border-radius: 999px; padding: 4px 8px 4px 12px; font-size: 12.5px; }
	.chip.add { background: transparent; border: 1px dashed var(--line); color: var(--muted); cursor: pointer; padding: 4px 12px; }
	.chip.add:hover { color: var(--fg); border-color: var(--accent); }
	.remove { background: transparent; border: none; color: inherit; opacity: 0.6; cursor: pointer; font-size: 15px; line-height: 1; padding: 0 2px; }
	.remove:hover { opacity: 1; }
	select { background: var(--bg); color: var(--fg); border: 1px solid var(--accent); border-radius: 8px; padding: 4px 8px; font-size: 12.5px; }
	.empty-hint { color: var(--muted); font-size: 12px; }
</style>
