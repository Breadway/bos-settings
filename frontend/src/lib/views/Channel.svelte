<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import LogView from "$lib/components/LogView.svelte";

	interface BakeryTrack {
		current: string;
		tracks: string[];
	}

	const LABELS: Record<string, string> = { stable: "Stable", beta: "Beta", dev: "Development" };
	const BLURBS: Record<string, string> = {
		stable: "Finished releases. The safest choice.",
		beta: "Releases being tested before they become stable.",
		dev: "Every change as soon as it is made. Newest, and sometimes broken.",
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
			message = `Switched to ${LABELS[name] ?? name}. Update bread apps to get its builds.`;
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

<ViewScaffold title="Update channel">
	<Group title="Bread apps channel" hint="How new the bread apps you get are. Nothing changes until you update.">
		{#if !track}
			<p class="muted">Loading…</p>
		{:else}
			<div class="ui-seg">
				{#each track.tracks as name (name)}
					<button class:on={track.current === name} onclick={() => setTrack(name)}>{LABELS[name] ?? name}</button>
				{/each}
			</div>
			<p class="muted">{BLURBS[track.current] ?? ""}</p>
		{/if}
		{#if message}<p class="muted" role="status">{message}</p>{/if}
		<div class="foot"><button class="btn small" disabled={busy} onclick={updateAll}>{busy ? "Updating…" : "Update bread apps now"}</button></div>
		{#if log.length}<details class="ui-disclosure"><summary>Details</summary><LogView lines={log} /></details>{/if}
	</Group>
</ViewScaffold>
<style>
	.muted { color:var(--muted); font-size:12.5px; margin:8px 2px; }
	.foot { padding:2px 0 10px; }
</style>
