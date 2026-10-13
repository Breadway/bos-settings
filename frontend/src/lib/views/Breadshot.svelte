<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import { open } from "@tauri-apps/plugin-dialog";
	import { go } from "$lib/nav.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";

	interface ShotBind {
		shortcut: string;
		command: string;
	}
	interface BreadshotConfig {
		save_dir: string;
		silent: boolean;
		freeze: boolean;
		notif_timeout: number;
		date_format: string;
	}

	let binds = $state<ShotBind[] | null>(null);
	let cfg = $state<BreadshotConfig | null>(null);

	onMount(async () => {
		binds = await invoke<ShotBind[]>("get_breadshot_binds");
		cfg = await invoke<BreadshotConfig>("get_breadshot_config");
	});

	async function save() {
		await invoke("save_breadshot_config", { cfg });
	}
	useDraft(() => (cfg), save, () => cfg !== null);

	async function pickFolder() {
		const picked = await open({ directory: true });
		if (typeof picked === "string") cfg!.save_dir = picked;
	}
	// A real example name instead of the raw strftime pattern.
	function example(fmt: string): string {
		const d = new Date();
		const p = (n: number) => String(n).padStart(2, "0");
		const map: Record<string, string> = { Y: String(d.getFullYear()), m: p(d.getMonth() + 1), d: p(d.getDate()), H: p(d.getHours()), M: p(d.getMinutes()), S: p(d.getSeconds()), "%": "%" };
		return `${(fmt || "%Y-%m-%d-%H%M%S").replace(/%(.)/g, (_, c) => map[c] ?? `%${c}`)}_breadshot.png`;
	}
</script>

<ViewScaffold title="Screenshots">
	{#if cfg}
		<Group title="Screenshots">
			<Row label="Save to" hint={cfg.save_dir || "~/Pictures/Screenshots"}>
				<button class="btn small" onclick={pickFolder}>Change</button>
			</Row>
			<SwitchField label="Show a notification" hint="With a preview after each screenshot." bind:value={() => !cfg!.silent, (v) => (cfg!.silent = !v)} />
			<SwitchField label="Freeze the screen while selecting" hint="Handy for menus and videos that would change." bind:value={cfg.freeze} />
			<Row label="Shortcuts" hint={binds?.length ? `${binds.length} set up. Change them in Keyboard.` : "Change them in Keyboard."}>
				<button class="btn small ghost" onclick={() => go("keyboard")}>Open</button>
				<button class="btn small" onclick={() => invoke("breadshot_region_clipboard")}>Take one now</button>
			</Row>
			<details class="ui-disclosure">
				<summary>Advanced</summary>
				{#if !cfg.silent}
					<Row label="Notification stays for">
						<select class="ui-input" bind:value={cfg.notif_timeout}>
							{#each [...new Set([2000, 3000, 5000, 10000, cfg.notif_timeout])].sort((a, b) => a - b) as ms (ms)}<option value={ms}>{ms / 1000} seconds</option>{/each}
						</select>
					</Row>
				{/if}
				<Row label="File name" hint={`Looks like ${example(cfg.date_format)}`}>
					<input class="ui-input" bind:value={cfg.date_format} placeholder="%Y-%m-%d-%H%M%S" />
				</Row>
			</details>
		</Group>
	{/if}
</ViewScaffold>
