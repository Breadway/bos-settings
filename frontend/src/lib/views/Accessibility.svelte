<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface A11yStatus {
		orca_installed: boolean;
		orca_running: boolean;
		zoom_factor: number;
		sticky_keys_supported: boolean;
		slow_keys_supported: boolean;
		kmag_installed: boolean;
		note: string;
	}

	let st = $state<A11yStatus | null>(null);
	let zoom = $state(1);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");

	async function refresh() {
		st = await invoke<A11yStatus>("get_a11y_status");
		zoom = st.zoom_factor;
	}

	onMount(refresh);

	async function install(packages: string[]) {
		log = [];
		busy = true;
		await runStreamed("pacman_install", { packages }, (line) => {
			log = [...log, line];
		});
		busy = false;
		await refresh();
	}

	async function toggleOrca(running: boolean) {
		message = "";
		try {
			await invoke("set_orca_running", { running });
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
	}

	async function applyZoom() {
		message = "";
		try {
			zoom = await invoke<number>("set_cursor_zoom", { factor: zoom });
		} catch (e) {
			message = `${e}`;
		}
	}

	async function openKmag() {
		message = "";
		try {
			await invoke("open_kmag");
		} catch (e) {
			message = `${e}`;
		}
	}
</script>

<ViewScaffold title="Accessibility">
	<Group title="Screen reader" hint="Orca. Works on Wayland; start/stop is process-level, not a compositor setting.">
		{#if !st}
			<Hint text="Loading…" />
		{:else if !st.orca_installed}
			<Hint text="orca is not installed." />
			<button class="primary" disabled={busy} onclick={() => install(["orca"])}>Install orca</button>
		{:else}
			<div class="row-switch">
				<span>Orca</span>
				<button
					class="switch"
					class:on={st.orca_running}
					role="switch"
					aria-checked={st.orca_running}
					aria-label="Orca"
					onclick={() => toggleOrca(!st!.orca_running)}
				>
					<span class="knob"></span>
				</button>
			</div>
		{/if}
	</Group>

	<Group title="Magnifier" hint="Hyprland’s real magnifier is cursor:zoom_factor — pointer-centered zoom. Applies to this session.">
		{#if st}
			<NumberField label="Zoom factor" bind:value={zoom} min={1} max={8} step={0.25} />
			<button class="primary" onclick={applyZoom}>Apply zoom</button>
			<Hint text="1.0 is off. Values above 1 enlarge around the cursor." />
			{#if !st.kmag_installed}
				<Hint text="kmag is a separate KDE magnifier and is not wired into Hyprland. Optional install only." />
				<button disabled={busy} onclick={() => install(["kmag"])}>Install kmag</button>
			{:else}
				<button onclick={openKmag}>Open kmag</button>
			{/if}
		{/if}
	</Group>

	<Group title="Sticky keys / Slow keys">
		{#if st}
			<div class="row-switch disabled">
				<span>Sticky keys</span>
				<button class="switch" disabled role="switch" aria-checked="false" aria-label="Sticky keys">
					<span class="knob"></span>
				</button>
			</div>
			<div class="row-switch disabled">
				<span>Slow keys</span>
				<button class="switch" disabled role="switch" aria-checked="false" aria-label="Slow keys">
					<span class="knob"></span>
				</button>
			</div>
			<Hint text={st.note} />
		{/if}
	</Group>
	{#if message}<Hint text={message} />{/if}
	<LogView lines={log} />
</ViewScaffold>

<style>
	.row-switch {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.row-switch.disabled {
		opacity: 0.55;
	}

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

	.switch:disabled {
		cursor: not-allowed;
	}

	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		align-self: flex-start;
		margin-top: var(--space-xs, 4px);
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
