<script lang="ts">
	import { open } from "@tauri-apps/plugin-dialog";
	import Plus from "@lucide/svelte/icons/plus";
	import X from "@lucide/svelte/icons/x";

	let { label, value = $bindable(), hint }: { label: string; value: string[]; hint?: string } = $props();

	async function addFolder() {
		const picked = await open({ directory: true });
		if (typeof picked === "string" && !value.includes(picked)) {
			value = [...value, picked];
		}
	}

	function remove(path: string) {
		value = value.filter((p) => p !== path);
	}

	function shorten(path: string): string {
		const home = "/home/";
		const idx = path.indexOf(home);
		if (idx === -1) return path;
		const afterHome = path.slice(idx + home.length);
		const slash = afterHome.indexOf("/");
		return slash === -1 ? path : `~/${afterHome.slice(slash + 1)}`;
	}
</script>

<div class="field">
	<div class="header">
		<span class="label">{label}</span>
		<button type="button" class="add" onclick={addFolder}>
			<Plus size={13} />
			Add folder…
		</button>
	</div>
	{#if hint}<p class="hint">{hint}</p>{/if}
	{#if value.length === 0}
		<p class="empty">None added.</p>
	{:else}
		<div class="chips">
			{#each value as path (path)}
				<span class="chip" title={path}>
					{shorten(path)}
					<button type="button" class="remove" onclick={() => remove(path)} aria-label={`Remove ${path}`}>
						<X size={12} />
					</button>
				</span>
			{/each}
		</div>
	{/if}
</div>

<style>
	.field {
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.label {
		font-weight: 500;
	}

	.hint {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
		margin: 4px 0 0;
	}

	.empty {
		opacity: 0.5;
		font-size: var(--font-size-secondary, 12px);
		margin: var(--space-sm, 8px) 0 0;
	}

	.add {
		display: flex;
		align-items: center;
		gap: 4px;
		background: transparent;
		color: var(--accent);
		border: none;
		font-size: var(--font-size-secondary, 12px);
		cursor: pointer;
		padding: 2px 4px;
	}

	.add:hover {
		text-decoration: underline;
	}

	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: var(--space-sm, 8px);
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
		font-family: monospace;
	}

	.remove {
		background: transparent;
		border: none;
		color: inherit;
		opacity: 0.6;
		cursor: pointer;
		display: flex;
		align-items: center;
		padding: 2px;
		border-radius: 50%;
	}

	.remove:hover {
		opacity: 1;
		background-color: color-mix(in srgb, var(--on-overlay) 15%, transparent);
	}
</style>
