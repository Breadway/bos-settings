<script lang="ts">
	let { value = $bindable(), ariaLabel }: { value: boolean; ariaLabel?: string } = $props();
</script>

<button
	type="button"
	class="switch"
	class:on={value}
	role="switch"
	aria-checked={value}
	aria-label={ariaLabel}
	onclick={() => (value = !value)}
>
	<i></i>
</button>

<style>
	.switch {
		width: 42px;
		height: 24px;
		flex-shrink: 0;
		border-radius: 999px;
		border: none;
		background: color-mix(in srgb, var(--fg) 12%, transparent);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
		transition: background 0.18s ease;
	}

	.switch.on {
		background: var(--accent);
	}

	i {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: #efe8e6;
		display: block;
		transform: translateX(0);
		transition: transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
	}

	.switch.on i {
		transform: translateX(18px);
		background: var(--on-accent);
	}
</style>
