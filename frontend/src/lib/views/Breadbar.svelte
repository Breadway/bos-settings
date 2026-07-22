<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
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
	let advancedOpen = $state(false);
	let css = $state("");
	let cssStatus = $state("");
	let cssSaving = $state(false);

	onMount(async () => {
		style = await invoke<BreadbarStyle>("get_breadbar_style");
		css = await invoke<string>("get_breadbar_css");
	});

	async function saveStyle() {
		await invoke("save_breadbar_style", { style });
		css = await invoke<string>("get_breadbar_css");
	}

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
		<Group title="Text" hint="Applies to the clock, workspace numbers, and stat labels.">
			<TextField label="Font" bind:value={style.font_family} placeholder="Varela Round" />
			<NumberField label="Font size" bind:value={style.font_size} min={8} max={32} />
		</Group>

		<Group title="Bar shape">
			<NumberField label="Corner rounding" bind:value={style.bar_border_radius} min={0} max={40} />
			<NumberField label="Inner padding" bind:value={style.bar_padding} min={0} max={40} />
		</Group>

		<Group title="Workspace indicator">
			<NumberField label="Size" bind:value={style.workspace_font_size} min={8} max={40} />
			<Row label="Inactive dimness">
				<div class="opacity-row">
					<input type="range" min="0" max="1" step="0.05" bind:value={style.workspace_inactive_opacity} />
					<span class="pct">{Math.round(style.workspace_inactive_opacity * 100)}%</span>
				</div>
			</Row>
		</Group>

		<Group title="Spacing & icons">
			<NumberField label="Gap between stats" bind:value={style.stat_gap} min={0} max={40} />
			<NumberField label="Tray icon size" bind:value={style.tray_icon_size} min={8} max={32} />
			<NumberField label="Notification corner rounding" bind:value={style.notification_border_radius} min={0} max={24} />
		</Group>

		<SaveButton onSave={saveStyle} />
	{/if}

	<Group title="Advanced" hint="Raw stylesheet. Anything set here can also be changed above — those fields edit this same file." wide>
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
	.opacity-row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
	}

	.opacity-row input[type="range"] {
		width: 120px;
	}

	.pct {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
		width: 4ch;
	}

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
