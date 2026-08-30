<script lang="ts">
	import type { Component } from "svelte";

	let { icon: Icon, title, hint }: { icon: Component; title: string; hint: string } = $props();
</script>

<div class="empty">
	<Icon size={40} />
	<span class="title">{title}</span>
	<span class="hint">{hint}</span>
</div>

<style>
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		padding: 28px 8px;
		color: var(--muted);
		text-align: center;
	}

	.title {
		font-weight: bold;
	}

	.hint {
		font-size: var(--font-size-secondary, 12px);
		max-width: 40ch;
	}
</style>
