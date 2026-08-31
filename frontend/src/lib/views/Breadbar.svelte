<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import { debounce } from "$lib/debounce";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";

	interface BreadbarStyle {
		font_family: string;
		font_size: number;
		bar_border_radius: number;
		bar_padding: number;
		workspace_inactive_opacity: number;
		workspace_font_size: number;
		stat_gap: number;
		tray_icon_size: number;
		notification_border_radius: number;
	}

	let style = $state<BreadbarStyle | null>(null);
	let loaded = $state(false);
	let advancedOpen = $state(false);
	let css = $state("");
	let cssStatus = $state("");
	let cssSaving = $state(false);

	const persist = debounce(() => {
		if (!style) return;
		invoke("save_breadbar_style", { style }).then(() => invoke<string>("get_breadbar_css").then((c) => (css = c)));
	}, 320);

	onMount(async () => {
		style = await invoke<BreadbarStyle>("get_breadbar_style");
		css = await invoke<string>("get_breadbar_css");
		loaded = true;
	});

	let primed = false;
	$effect(() => {
		if (!loaded || !style) return;
		JSON.stringify(style);
		if (!primed) {
			primed = true;
			return;
		}
		persist();
	});

	async function saveCss() {
		cssSaving = true;
		try {
			const reloaded = await invoke<boolean>("save_breadbar_css", { css });
			cssStatus = reloaded ? "Saved & reloaded" : "Saved";
			style = await invoke<BreadbarStyle>("get_breadbar_style");
			setTimeout(() => (cssStatus = ""), 3000);
		} catch (e) {
			cssStatus = `Error: ${e}`;
		} finally {
			cssSaving = false;
		}
	}
</script>

<ViewScaffold title="Bar">
	{#if style}
		<Group title="Text" hint="Clock, workspace numbers, and stats.">
			<TextField label="Font" bind:value={style.font_family} placeholder="Varela Round" />
			<SliderField label="Font size" bind:value={style.font_size} min={8} max={32} />
		</Group>

		<Group title="Bar shape">
			<SliderField label="Corner rounding" bind:value={style.bar_border_radius} min={0} max={40} />
			<SliderField label="Inner padding" bind:value={style.bar_padding} min={0} max={40} />
		</Group>

		<Group title="Workspaces">
			<SliderField label="Number size" bind:value={style.workspace_font_size} min={8} max={40} />
			<SliderField
				label="Inactive dimness"
				bind:value={style.workspace_inactive_opacity}
				min={0}
				max={1}
				step={0.05}
			/>
		</Group>

		<Group title="Spacing and icons">
			<SliderField label="Gap between stats" bind:value={style.stat_gap} min={0} max={40} />
			<SliderField label="Tray icon size" bind:value={style.tray_icon_size} min={8} max={32} />
			<SliderField label="Notification rounding" bind:value={style.notification_border_radius} min={0} max={24} />
		</Group>

		<Hint text="Applies as you change it." />
	{/if}

	<Group title="Advanced" hint="Raw CSS. Same file as the fields above." wide>
		<button class="toggle" onclick={() => (advancedOpen = !advancedOpen)}>
			{#if advancedOpen}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
			Edit raw CSS
		</button>

		{#if advancedOpen}
			<textarea bind:value={css} spellcheck="false"></textarea>
			<div class="row">
				<button class="save-css" disabled={cssSaving} onclick={saveCss}>Save CSS</button>
				<span class="status">{cssStatus}</span>
			</div>
		{/if}
	</Group>
</ViewScaffold>

<style>
	.toggle {
		display: flex;
		align-items: center;
		gap: var(--space-xs, 4px);
		background: transparent;
		border: none;
		color: var(--on-surface);
		opacity: 0.7;
		cursor: pointer;
		padding: var(--space-xs, 4px) 0;
	}

	.toggle:hover {
		opacity: 1;
	}

	textarea {
		width: 100%;
		min-height: 360px;
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px);
		font-family: monospace;
		resize: vertical;
		margin-top: var(--space-sm, 8px);
	}

	textarea:focus {
		outline: none;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		margin-top: var(--space-md, 12px);
	}

	.save-css {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}

	.save-css:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
