<script lang="ts">
	import { onMount } from "svelte";
	import {recordChange} from "$lib/changes.svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";

	interface ShellThemeInfo {
		id: string;
		name: string;
		source: string; // "builtin" | "system" | "user"
	}

	let themes = $state<ShellThemeInfo[]>([]);
	let active = $state<string>("");
	let loaded = $state(false);
	let busy = $state(false);
	let status = $state("");
	// Set once the user switches, so the restart prompt gains emphasis.
	// breadbar/breadbox don't hot-reload a shell-theme change today; it
	// takes effect when they restart or on the next login.
	let switched = $state(false);

	onMount(async () => {
		[themes, active] = await Promise.all([
			invoke<ShellThemeInfo[]>("list_shell_themes"),
			invoke<string>("get_active_shell_theme"),
		]);
		loaded = true;
	});

	async function select(id: string) {
		if (id === active || busy) return;
		const prev = active;
		active = id;
		busy = true;
		status = "Applying…";
		try {
			await recordChange(`Shell style: ${themes.find(t=>t.id===id)?.name ?? id}`,()=>invoke<void>("set_active_shell_theme", { id }),async()=>{await invoke("set_active_shell_theme",{id:prev});active=prev;});
			switched = true;
			status = "Applied";
			setTimeout(() => (status = ""), 3000);
		} catch (e) {
			active = prev;
			status = `${e}`;
		} finally {
			busy = false;
		}
	}

	async function restart() {
		busy = true;
		status = "Restarting…";
		try {
			await invoke("restart_shell_apps");
			switched = false;
			status = "Restarted";
			setTimeout(() => (status = ""), 3000);
		} catch (e) {
			status = `${e}`;
		} finally {
			busy = false;
		}
	}
</script>

<ViewScaffold
	title="Shell theme"
	lede="The layout and motion style for the bar and launcher. Colors come from the wallpaper."
>
	<Group title="Theme" wide>
		{#if !loaded}
			<Hint text="Loading…" />
		{:else}
			<div class="cards">
				{#each themes as t (t.id)}
					<button
						type="button"
						class="card"
						class:on={active === t.id}
						disabled={busy}
						onclick={() => select(t.id)}
					>
						<span class="preview" data-theme={t.id} class:light={/daylight/.test(t.id)} aria-hidden="true"><i class="mini-bar"></i><i class="mini-window"></i></span>
						<span class="meta">
							<span class="name">{t.name}</span>
							<span class="sub">
								{#if t.source !== "builtin"}Custom theme{/if}
							</span>
						</span>
					</button>
				{/each}
			</div>
			<div class="foot">
				<span class="status">{status || (switched ? "The bar has switched. The launcher changes the next time you open it." : "")}</span>
				<button class="btn small ghost" disabled={busy} onclick={restart} title="If something looks stuck after a change">Restart bar and launcher</button>
			</div>
		{/if}
	</Group>

</ViewScaffold>

<style>
 .preview{position:relative;display:block;width:100%;height:72px;border-radius:8px;background:linear-gradient(140deg,var(--accent-soft),var(--bg));overflow:hidden}.preview .mini-bar{position:absolute;left:8px;right:8px;top:6px;height:8px;background:var(--accent);border-radius:5px} .preview[data-theme="spotlight"] .mini-bar{left:38%;right:38%;height:7px;background:#0a0c0ed9}.preview[data-theme="console"] .mini-bar{left:0;right:0;top:0;border-radius:0;background:#0b0d0b}.preview[data-theme="strata"] .mini-bar{left:0;right:auto;top:0;bottom:0;width:12px;height:auto;border-radius:0}.preview[data-theme="kuro"] .mini-bar{top:auto;bottom:0;left:0;right:0;border-radius:0;background:#000}.preview[data-theme="glass-workbench"] .mini-bar{background:#ffffff2e;border:1px solid #ffffff4d}.preview[data-theme="liquid-motion"] .mini-bar{top:auto;bottom:5px;left:20%;right:20%;height:10px;background:#ffffff33}.preview[data-theme="daylight"] .mini-bar{left:0;right:0;top:0;border-radius:0;background:#f2efe8}.preview[data-theme="loaf"] .mini-bar{left:30%;right:30%;background:#d9a066;border-radius:4px}.preview.light{background:linear-gradient(140deg,#e8dccb,#c9b8a0)}.mini-window{position:absolute;left:16px;right:16px;top:22px;bottom:16px;border:1px solid var(--line);border-radius:6px;background:var(--surface)}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 8px;
	}

	.card {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 10px;
		padding: 12px 14px;
		border-radius: 10px;
		text-align: left;
		background: var(--bg);
		border: 2px solid transparent;
		color: inherit;
		cursor: pointer;
	}

	.card:disabled {
		cursor: default;
		opacity: 0.7;
	}

	.card.on {
		border-color: var(--accent);
	}



	.meta {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.name {
		font-size: 14px;
	}

	.sub {
		font-size: 11px;
		color: var(--muted);
		text-transform: capitalize;
	}


	.foot {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 12px;
	}

	.status {
		color: var(--muted);
		font-size: 12px;
	}
</style>
