<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
	import { isSystemStartup } from "$lib/systemStartup";
	import Switch from "$lib/components/Switch.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Plus from "@lucide/svelte/icons/plus";
	import { changes } from "$lib/changes.svelte";
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import FolderOpen from "@lucide/svelte/icons/folder-open";

	interface AutostartEntry {
		command: string;
		label: string;
		enabled: boolean;
	}

	let apps=$state<{id:string;name:string}[]>([]);
	let picking=$state(false);
	let editing=$state<AutostartEntry|null>(null);
	let custom=$state<AutostartEntry|null>(null);
	// "gtk-launch org.gnome.Nautilus.desktop" -> "org.gnome.Nautilus"; a path -> its file name.
	function appName(command:string):string{const first=command.trim().split(/\s+/);const target=first[0]==="gtk-launch"?first[1]??"":first[0]??"";return target.replace(/\.desktop$/,"").split("/").pop()||"Command";}
	let appQuery=$state("");
	onMount(async()=>{apps=await invoke("list_startup_apps")});
	function addApp(app:{id:string;name:string}) { if(!/^[a-zA-Z0-9_.-]+\.desktop$/.test(app.id)) return; entries=[...(entries ?? []),{command:`gtk-launch ${app.id}`,label:app.name,enabled:true}];picking=false; changes.message=`${app.name} opens at login`; changes.undo=null; }
	let editSystem = $state(false);
	let originalSystem = "";
	let entries = $state<AutostartEntry[] | null>(null);

	onMount(async () => {
		entries = await invoke<AutostartEntry[]>("get_autostart_entries");
		originalSystem = JSON.stringify(entries.filter(e => isSystemStartup(e.command)));
	});

	function removeEntry(i: number) {
		entries = entries!.filter((_, idx) => idx !== i);
	}

	async function browseFor(entry: AutostartEntry) {
		const picked = await open({ directory: false, defaultPath: "/usr/bin" });
		if (typeof picked === "string") entry.command = picked;
	}

	async function save() {
		const system = JSON.stringify(entries?.filter(e => isSystemStartup(e.command)));
		const changed = system !== originalSystem;
		if (changed && !(await confirmAction({ title: "Change core desktop services?", message: "Disabling or removing them can stop locking, screenshots, wallpaper or other desktop features.", confirmLabel: "Save changes" }))) throw new Error("System service changes were not saved.");
		await invoke("save_autostart_entries", { entries, allowSystemChanges: changed });
		originalSystem = system;
	}
	useDraft(() => (entries), save, () => entries !== null);
</script>

<ViewScaffold title="Startup" lede="Apps that open when you log in.">
	{#if entries}
		<Group title="Your apps">
			{#each entries as entry, i (i)}
				{#if !isSystemStartup(entry.command)}
					<div class="ui-item">
						<div class="lb"><b>{entry.label || appName(entry.command)}</b><small title={entry.command}>{entry.enabled ? "Opens at login" : "Off"}</small></div>
						<button class="btn small ghost" onclick={() => (editing = entry)}>Edit</button>
						<Switch bind:value={entry.enabled} ariaLabel={`Open ${entry.label || "this"} at login`} />
					</div>
				{/if}
			{/each}
			{#if !entries.some((e) => !isSystemStartup(e.command))}<p class="empty">Nothing opens at login yet.</p>{/if}
			<div class="foot">
				<button class="btn small" onclick={() => { appQuery = ""; picking = true; }}><Plus size={14} />Add an app</button>
				<button class="btn small ghost" onclick={() => (custom = { command: "", label: "", enabled: true })}>Add a command</button>
			</div>
		</Group>

		<Group title="Desktop services">
			<details class="ui-disclosure">
				<summary>Started by BOS ({entries.filter((e) => isSystemStartup(e.command)).length})</summary>
				<p class="empty">These run the bar, locking, screenshots and other parts of the desktop. Leave them on unless you are replacing one.</p>
				{#each entries as entry, i (i)}
					{#if isSystemStartup(entry.command)}
						<div class="ui-item">
							<div class="lb"><b>{entry.label || entry.command}</b><small class:off={!entry.enabled}>{entry.enabled ? "Runs at login" : "Turned off"}</small>
								{#if editSystem}<input class="ui-input cmd" aria-label="Command" bind:value={entry.command} />{/if}
							</div>
							{#if editSystem}<Switch bind:value={entry.enabled} ariaLabel={entry.label || entry.command} /><button class="btn small remove" onclick={() => removeEntry(i)}>Remove</button>{/if}
						</div>
					{/if}
				{/each}
				{#if !editSystem}
					<div class="foot"><button class="btn small ghost" onclick={async () => { if (await confirmAction({ title: "Change desktop services?", message: "Turning one off can stop locking, screenshots, the wallpaper or other parts of the desktop.", confirmLabel: "Change them" })) editSystem = true; }}>Change these…</button></div>
				{/if}
			</details>
		</Group>
	{/if}
</ViewScaffold>

{#if picking}
	<Sheet title="Add a startup app" onClose={() => (picking = false)}>
		<input class="ui-input" aria-label="Search installed apps" bind:value={appQuery} placeholder="Search installed apps" />
		<div class="picker">
			{#each apps.filter((a) => a.name.toLowerCase().includes(appQuery.toLowerCase())) as app (app.id)}
				<div class="ui-item"><div class="lb"><b>{app.name}</b></div><button class="btn small primary" onclick={() => addApp(app)}>Add</button></div>
			{/each}
		</div>
		{#snippet actions()}<button class="btn" onclick={() => (picking = false)}>Done</button>{/snippet}
	</Sheet>
{/if}

{#if editing}
	{@const entry = editing}
	<Sheet title={entry.label || "Startup command"} onClose={() => (editing = null)}>
		<label class="field"><span>Name</span><input class="ui-input" bind:value={entry.label} placeholder={appName(entry.command)} /></label>
		<label class="field"><span>Command</span>
			<span class="cmdrow"><input class="ui-input" bind:value={entry.command} /><button class="btn small ghost" onclick={() => browseFor(entry)} aria-label="Pick a program"><FolderOpen size={14} /></button></span>
		</label>
		{#snippet actions()}
			<button class="btn remove" onclick={() => { removeEntry(entries!.indexOf(entry)); editing = null; }}>Remove</button>
			<span class="spacer"></span>
			<button class="btn primary" onclick={() => (editing = null)}>Done</button>
		{/snippet}
	</Sheet>
{/if}

{#if custom}
	{@const c = custom}
	<Sheet title="Add a command" lede="Runs once each time you log in." onClose={() => (custom = null)}>
		<label class="field"><span>Name</span><input class="ui-input" bind:value={c.label} placeholder="What it is, e.g. Sync notes" /></label>
		<label class="field"><span>Command</span>
			<span class="cmdrow"><input class="ui-input" bind:value={c.command} placeholder="e.g. syncthing --no-browser" /><button class="btn small ghost" onclick={() => browseFor(c)} aria-label="Pick a program"><FolderOpen size={14} /></button></span>
		</label>
		{#snippet actions()}
			<button class="btn" onclick={() => (custom = null)}>Cancel</button>
			<button class="btn primary" disabled={!c.command.trim()} onclick={() => { entries = [...entries!, { ...c, command: c.command.trim() }]; custom = null; }}>Add</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.empty { color:var(--muted); font-size:12.5px; margin:6px 2px 8px; line-height:1.45; }
	.foot { display:flex; gap:8px; padding:6px 0 10px; flex-wrap:wrap; }
	.foot .btn { display:inline-flex; align-items:center; gap:6px; }
	.ui-item small.off { color:var(--red); }
	.cmd { width:100%; margin-top:6px; font-size:12px; }
	.picker { max-height:340px; overflow:auto; margin-top:8px; }
	.field { display:flex; flex-direction:column; gap:6px; padding:8px 0; font-size:13px; }
	.cmdrow { display:flex; gap:6px; }
	.cmdrow input { flex:1; }
	.spacer { flex:1; }
</style>
