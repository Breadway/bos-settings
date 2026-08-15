<script lang="ts">
	let { page }: { page: string } = $props();
</script>

<div class="placeholder">
	<p>Unknown page "{page}".</p>
</div>

<style>
	.placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		opacity: 0.5;
	}
</style>
