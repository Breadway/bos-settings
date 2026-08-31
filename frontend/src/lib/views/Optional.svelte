<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface OptionalItem {
		id: string;
		title: string;
		detail: string;
		installed: boolean;
		via: string;
	}
	interface OptionalStatus {
		items: OptionalItem[];
		flathub: boolean;
	}

	let st = $state<OptionalStatus | null>(null);
	let log = $state<string[]>([]);
	let busy = $state(false);
	let message = $state("");

	async function refresh() {
		st = await invoke<OptionalStatus>("get_optional_software");
	}

	onMount(refresh);

	async function stream(command: string, args: Record<string, unknown> = {}) {
		log = [];
		busy = true;
		message = "";
		const ok = await runStreamed(command, args, (line) => {
			log = [...log, line];
		});
		busy = false;
		if (!ok) message = "Install failed. See the log.";
		await refresh();
		return ok;
	}

	async function install(id: string) {
		if (id === "breadcast") {
			await stream("bakery_install", { name: "breadcast" });
			return;
		}
		if (id === "flatpak") {
			const ok = await stream("pacman_install", { packages: ["flatpak"] });
			if (ok) {
				try {
					await invoke("enable_flathub");
					message = "Flathub user remote added.";
				} catch (e) {
					message = `${e}`;
				}
				await refresh();
			}
			return;
		}
		if (id === "office") {
			await stream("pacman_install", { packages: ["libreoffice-fresh", "papers"] });
			return;
		}
		if (id === "steam") {
			await stream("pacman_install", { packages: ["steam"] });
		}
	}
</script>

<ViewScaffold title="Optional software">
	<Group
		title="Curated extras"
		hint="Not an AUR browser. breadcast is bakery-only and is not on the ISO. Pacman items use the allowlisted installer."
		wide
	>
		{#if !st}
			<Hint text="Loading…" />
		{:else}
			{#each st.items as item (item.id)}
				<div class="card">
					<div class="meta">
						<strong>{item.title}</strong>
						<p>{item.detail}</p>
						<span class="via">{item.via}{item.installed ? " · installed" : ""}</span>
					</div>
					{#if item.installed}
						<span class="done">Installed</span>
					{:else}
						<button class="primary" disabled={busy} onclick={() => install(item.id)}>Install</button>
					{/if}
				</div>
			{/each}
		{/if}
		{#if message}<Hint text={message} />{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>

<style>
	.card {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		margin-bottom: var(--space-sm, 8px);
	}

	.meta {
		flex: 1;
		min-width: 0;
	}

	.meta p {
		margin: 4px 0;
		opacity: 0.8;
		font-size: var(--font-size-secondary, 12px);
	}

	.via,
	.done {
		opacity: 0.65;
		font-size: var(--font-size-secondary, 12px);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
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
