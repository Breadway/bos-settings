<script lang="ts">
	let { label, value = $bindable() }: { label: string; value: string[] } = $props();

	let text = $state("");

	function commit() {
		const v = text.trim();
		if (v && !value.includes(v)) {
			value = [...value, v];
		}
		text = "";
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" || e.key === ",") {
			e.preventDefault();
			commit();
		} else if (e.key === "Backspace" && text === "" && value.length > 0) {
			value = value.slice(0, -1);
		}
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
		<input type="text" bind:value={text} onkeydown={onKeydown} placeholder="Type and press Enter…" />
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

	input {
		flex: 1;
		min-width: 12ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: 4px var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
