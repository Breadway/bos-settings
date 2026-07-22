<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import Row from "./Row.svelte";
	import FolderOpen from "@lucide/svelte/icons/folder-open";

	let {
		label,
		value = $bindable(),
		placeholder = "",
		mode = "file",
		extensions,
	}: { label: string; value: string; placeholder?: string; mode?: "file" | "folder"; extensions?: string[] } =
		$props();

	async function browse() {
		const picked = await open({
			directory: mode === "folder",
			filters: extensions ? [{ name: "Files", extensions }] : undefined,
		});
		if (typeof picked === "string") value = picked;
	}
</script>

<Row {label}>
	<div class="wrap">
		<input type="text" bind:value {placeholder} />
		<button type="button" onclick={browse} aria-label="Browse…">
			<FolderOpen size={14} />
		</button>
	</div>
</Row>

<style>
	.wrap {
		display: flex;
		gap: var(--space-xs, 4px);
	}

	input {
		width: 22ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
		cursor: pointer;
		display: flex;
		align-items: center;
	}

	button:hover {
		background-color: color-mix(in srgb, var(--on-surface) 10%, transparent);
	}
</style>
