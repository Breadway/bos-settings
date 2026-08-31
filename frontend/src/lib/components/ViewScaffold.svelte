<script lang="ts">
	import type { Snippet } from "svelte";
	import { getContext } from "svelte";
	import { EMBEDDED_KEY } from "$lib/embed";

	let { title, lede, children }: { title: string; lede?: string; children: Snippet } = $props();
	const embedded = getContext<boolean>(EMBEDDED_KEY) ?? false;
</script>

{#if embedded}
	<div class="embed">
		{@render children()}
	</div>
{:else}
	<div class="view">
		<h1 class="title">{title}</h1>
		{#if lede}
			<p class="lede">{lede}</p>
		{/if}
		<div class="content">
			{@render children()}
		</div>
	</div>
{/if}

<style>
	.view {
		padding: 28px 36px 56px;
		min-height: 100%;
	}

	.title {
		font-size: 28px;
		font-weight: 600;
		letter-spacing: -0.04em;
		margin: 0 auto;
		max-width: 920px;
		width: 100%;
	}

	.lede {
		margin: 6px auto 0;
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		font-size: 14px;
		line-height: 1.45;
		max-width: 920px;
		width: 100%;
	}

	.content,
	.embed {
		display: flex;
		flex-direction: column;
		gap: 18px;
		max-width: 920px;
		width: 100%;
		margin-top: 22px;
		margin-inline: auto;
	}

	.embed {
		margin-top: 0;
	}
</style>
