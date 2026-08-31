<script lang="ts">
	let { text }: { text: string } = $props();
</script>

<p class="hint">{text}</p>

<style>
	.hint {
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		font-size: var(--font-size-secondary, 12px);
		margin: 0;
		line-height: 1.4;
	}
</style>
