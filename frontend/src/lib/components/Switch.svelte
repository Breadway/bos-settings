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
	<span class="knob"></span>
</button>

<style>
	.switch {
		width: 40px;
		height: 22px;
		flex-shrink: 0;
		border-radius: 999px;
		border: none;
		background-color: var(--overlay);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
	}

	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}

	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}
</style>
