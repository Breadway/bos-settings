<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface ImePackage {
		name: string;
		installed: boolean;
	}
	interface ImeStatus {
		enabled: boolean;
		running: boolean;
		packages: ImePackage[];
		error: string | null;
	}

	let st = $state<ImeStatus | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");

	const installSet = ["fcitx5-im", "fcitx5-gtk", "fcitx5-qt", "fcitx5-chinese-addons"];

	async function refresh() {
		st = await invoke<ImeStatus>("get_ime_status");
	}

	onMount(refresh);

	async function toggle(enabled: boolean) {
		message = "";
		try {
			st = await invoke<ImeStatus>("set_ime_enabled", { enabled });
		} catch (e) {
			message = `${e}`;
		}
	}

	async function installMissing() {
		const missing = st?.packages.filter((p) => !p.installed).map((p) => p.name) ?? [];
		const packages = ["fcitx5-im", ...missing.filter((n) => n !== "fcitx5")];
		const unique = [...new Set(packages.length ? packages : installSet)];
		log = [];
		busy = true;
		await runStreamed("pacman_install", { packages: unique }, (line) => {
			log = [...log, line];
		});
		busy = false;
		await refresh();
	}

	let missing = $derived(st?.packages.filter((p) => !p.installed) ?? []);
</script>

<ViewScaffold title="Input method">
	<Group
		title="fcitx5"
		hint="Starts fcitx5. Running apps need a logout."
	>
		{#if !st}
			<Hint text="Loading…" />
		{:else}
			<div class="row-switch">
				<span>Enable fcitx5 for this session</span>
				<button
					class="switch"
					class:on={st.enabled}
					role="switch"
					aria-checked={st.enabled}
					aria-label="Enable fcitx5"
					onclick={() => toggle(!st!.enabled)}
				>
					<span class="knob"></span>
				</button>
			</div>
			<Hint text={st.running ? "fcitx5 is running." : "fcitx5 is not running."} />
			<button onclick={() => invoke("open_fcitx_config")}>Open fcitx5 config</button>
		{/if}
		{#if message}<Hint text={message} />{/if}
	</Group>

	<Group title="Packages" hint="fcitx5-im (group) plus GTK/Qt modules and a CJK table (fcitx5-chinese-addons).">
		{#if st}
			<ul>
				{#each st.packages as p (p.name)}
					<li class:missing={!p.installed}>{p.name}{p.installed ? "" : " (missing)"}</li>
				{/each}
			</ul>
		{/if}
		{#if missing.length}
			<button class="primary" disabled={busy} onclick={installMissing}>Install missing</button>
		{/if}
	</Group>
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

	ul {
		margin: 0 0 var(--space-sm, 8px);
		padding-left: 1.2em;
	}

	.missing {
		opacity: 0.7;
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		align-self: flex-start;
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
