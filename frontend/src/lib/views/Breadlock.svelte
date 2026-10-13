<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import Lock from "@lucide/svelte/icons/lock";
	import SelectField from "$lib/components/SelectField.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface BreadlockConfig {
		background_mode: string;
		background_path: string;
		background_blur: boolean;
		clock_format: string;
		font_family: string;
		fail_timeout_ms: number;
	}

	let cfg = $state<BreadlockConfig | null>(null);

	// Plain choices for the raw breadlock.toml values; anything custom stays selectable.
	const CLOCKS: Record<string, string> = { "%H:%M": "24-hour", "%H:%M:%S": "24-hour with seconds", "%I:%M %p": "12-hour", "%I:%M:%S %p": "12-hour with seconds" };
	const WAITS = [500, 1000, 2000];
	const waitLabel = (ms: number) => (ms === 0 ? "Don't wait" : ms < 1000 ? `${ms / 1000} seconds`.replace(/^0\.5 seconds$/, "Half a second") : ms === 1000 ? "1 second" : `${ms / 1000} seconds`);
	const waitOptions = (cur: number) => Array.from(new Set([...WAITS, cur])).sort((a, b) => a - b).map(String);
	let examplePath = $state<string | null>(null);
	let custom = $state(false);
	let twelve = $derived(cfg?.clock_format.includes("%I") ?? false);
	let seconds = $derived(cfg?.clock_format.includes("%S") ?? false);
	$effect(() => { if (cfg && !(cfg.clock_format in CLOCKS)) custom = true; });
	function setClock(h12: boolean, secs: boolean) {
		cfg!.clock_format = h12 ? (secs ? "%I:%M:%S %p" : "%I:%M %p") : secs ? "%H:%M:%S" : "%H:%M";
	}
	async function pickImage() {
		const picked = await open({ filters: [{ name: "Pictures", extensions: ["png"] }] });
		if (typeof picked === "string") cfg!.background_path = picked;
	}

	onMount(async () => {
		cfg = await invoke<BreadlockConfig>("get_breadlock_config");
		examplePath = await invoke<string | null>("breadlock_example_path");
	});

	async function save() {
		await invoke("save_breadlock_config", { cfg });
	}
	useDraft(() => (cfg), save, () => cfg !== null);
</script>

<ViewScaffold title="Lock screen" lede="What you see when the screen locks.">
	{#if cfg}
		<Group title="Look">
			<Row label="Background">
				<div class="ui-seg">
					<button class:on={cfg.background_mode === "image"} onclick={() => (cfg!.background_mode = "image")}>Picture</button>
					<button class:on={cfg.background_mode === "color"} onclick={() => (cfg!.background_mode = "color")}>Colour</button>
				</div>
			</Row>
			{#if cfg.background_mode === "image"}
				<Row label="Picture" hint={cfg.background_path ? cfg.background_path.split("/").pop() : "None chosen"}>
					<button class="btn small" onclick={pickImage}>Choose…</button>
				</Row>
			{/if}
			{#if custom}
				<Row label="Clock" hint="Custom format">
					<input class="ui-input" bind:value={cfg.clock_format} placeholder="%H:%M" />
					<button class="btn small ghost" onclick={() => (cfg!.clock_format = "%H:%M")}>Reset</button>
				</Row>
			{:else}
				<Row label="Clock">
					<div class="ui-seg">
						<button class:on={!twelve} onclick={() => setClock(false, seconds)}>24-hour</button>
						<button class:on={twelve} onclick={() => setClock(true, seconds)}>12-hour</button>
					</div>
				</Row>
				<SwitchField label="Show seconds" bind:value={() => seconds, (v) => setClock(twelve, v)} />
			{/if}
			<Row label="After a wrong password, wait">
				<div class="ui-seg">
					{#each waitOptions(cfg.fail_timeout_ms) as ms (ms)}<button class:on={cfg.fail_timeout_ms === Number(ms)} onclick={() => (cfg!.fail_timeout_ms = Number(ms))}>{waitLabel(Number(ms))}</button>{/each}
				</div>
			</Row>
			<SelectField label="Font" bind:value={cfg.font_family} options={[...new Set([cfg.font_family, "Varela Round", "Outfit", "Inter", "Noto Sans"].filter(Boolean))]} />
		</Group>
		<div class="foot"><button class="btn" onclick={() => invoke("lock_session")}><Lock size={14} />Lock now to preview</button></div>
		<details class="ui-disclosure">
			<summary>Advanced</summary>
			<div class="ui-item">
				<div class="lb"><b>Clock format</b><small>Write your own, like %A %H:%M for the weekday too.</small></div>
				<button class="btn small ghost" onclick={() => (custom = true)}>Customise</button>
			</div>
			<div class="ui-item">
				<div class="lb"><b>Settings file</b><small>Every lock screen option, in a text editor.</small></div>
				<button class="btn small ghost" onclick={() => invoke("open_breadlock_config")}>Open</button>
				{#if examplePath}<button class="btn small ghost" onclick={() => invoke("open_breadlock_example")}>Example</button>{/if}
			</div>
		</details>
	{/if}
</ViewScaffold>

<style>
	.foot { display:flex; }
	.foot .btn { display:inline-flex; align-items:center; gap:6px; }
</style>
