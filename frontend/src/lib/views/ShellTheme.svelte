<script lang="ts">
	import { onMount } from "svelte";
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
	// breadbar/breadbox don't hot-reload a shell-theme change today — it
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
			await invoke("set_active_shell_theme", { id });
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
						<span class="dot" aria-hidden="true"></span>
						<span class="meta">
							<span class="name">{t.name}</span>
							<span class="sub">
								<span class="id">{t.id}</span>{#if t.source !== "builtin"} · {t.source}{/if}
							</span>
						</span>
					</button>
				{/each}
			</div>
			<div class="foot">
				<span class="status">{status}</span>
			</div>
		{/if}
	</Group>

	{#if loaded}
		<Group title="Apply">
			<Hint
				text={switched
					? "Restart the bar and launcher (or log out and back in) to switch to this theme."
					: "A theme change takes effect when the bar and launcher restart, or on the next login."}
			/>
			<div class="foot">
				<button class="btn primary" disabled={busy} onclick={restart}>
					Restart bar &amp; launcher
				</button>
			</div>
		</Group>
	{/if}
</ViewScaffold>

<style>
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 8px;
	}

	.card {
		display: flex;
		align-items: center;
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

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 999px;
		flex-shrink: 0;
		background: color-mix(in srgb, var(--fg) 25%, transparent);
	}

	.card.on .dot {
		background: var(--accent);
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

	.sub .id {
		text-transform: none;
		font-family: var(--font-mono, ui-monospace, monospace);
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
