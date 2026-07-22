<script lang="ts">
	let { page }: { page: string } = $props();
</script>

<div class="placeholder">
	<p>"{page}" hasn't been migrated to Tauri yet.</p>
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
