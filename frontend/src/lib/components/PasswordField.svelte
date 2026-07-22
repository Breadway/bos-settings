<script lang="ts">
	import Row from "./Row.svelte";
	import Eye from "@lucide/svelte/icons/eye";
	import EyeOff from "@lucide/svelte/icons/eye-off";

	let { label, value = $bindable() }: { label: string; value: string } = $props();

	let reveal = $state(false);
</script>

<Row {label}>
	<div class="wrap">
		<input type={reveal ? "text" : "password"} bind:value />
		<button type="button" class="peek" onclick={() => (reveal = !reveal)}>
			{#if reveal}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
		</button>
	</div>
</Row>

<style>
	.wrap {
		display: flex;
		align-items: center;
		width: 28ch;
		background-color: var(--bg);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding-inline-end: var(--space-xs, 4px);
	}

	.wrap:focus-within {
		border-color: var(--accent);
	}

	input {
		flex: 1;
		min-width: 0;
		background: transparent;
		color: var(--on-surface);
		border: none;
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
	}

	.peek {
		background: transparent;
		border: none;
		color: var(--on-surface);
		opacity: 0.6;
		cursor: pointer;
		display: flex;
	}

	.peek:hover {
		opacity: 1;
	}
</style>
