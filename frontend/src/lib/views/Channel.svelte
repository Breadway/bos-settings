<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface BakeryTrack {
		current: string;
		tracks: string[];
	}

	const BLURBS: Record<string, string> = {
		stable: "Last tagged release.",
		beta: "Latest release candidate (vX.Y.Z-rc.N).",
		dev: "Every push to main.",
	};

	let track = $state<BakeryTrack | null>(null);
	let message = $state("");
	let log = $state<string[]>([]);
	let busy = $state(false);

	async function refresh() {
		try {
			track = await invoke<BakeryTrack>("get_bakery_track");
		} catch (e) {
			message = `${e}`;
		}
	}

	onMount(refresh);

	async function setTrack(name: string) {
		message = "";
		try {
			track = await invoke<BakeryTrack>("set_bakery_track", { track: name });
			message = `Now on ${name}. Run bakery update --all to install this track’s builds.`;
		} catch (e) {
			message = `${e}`;
		}
	}

	async function updateAll() {
		log = [];
		busy = true;
		await runStreamed("bakery_update_all", {}, (line) => {
			log = [...log, line];
		});
		busy = false;
	}
</script>

<ViewScaffold title="Bakery channel">
	<Group
		title="Track"
		hint="Nothing downloads until you update."
	>
		{#if !track}
			<Hint text="Loading…" />
		{:else}
			<div class="tracks">
				{#each track.tracks as name (name)}
					<button class="track" class:on={track.current === name} onclick={() => setTrack(name)}>
						<strong>{name}</strong>
						<span>{BLURBS[name] ?? ""}</span>
					</button>
				{/each}
			</div>
			<Hint text={`Current track: ${track.current}`} />
		{/if}
		<button class="primary" disabled={busy} onclick={updateAll}>Update all on this track</button>
		{#if message}<Hint text={message} />{/if}
	</Group>
	<LogView lines={log} />
</ViewScaffold>

<style>
	.tracks {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.track {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
		background-color: var(--surface);
		color: inherit;
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-md, 12px) var(--space-lg, 16px);
		cursor: pointer;
		text-align: left;
		font: inherit;
	}

	.track span {
		opacity: 0.7;
		font-size: var(--font-size-secondary, 12px);
	}

	.track.on {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
