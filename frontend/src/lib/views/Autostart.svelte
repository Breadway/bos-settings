<script lang="ts">
	import { isSystemStartup } from "$lib/systemStartup";
	import Hint from "$lib/components/Hint.svelte";
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { open } from "@tauri-apps/plugin-dialog";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import FolderOpen from "@lucide/svelte/icons/folder-open";

	interface AutostartEntry {
		command: string;
		label: string;
		enabled: boolean;
	}

	let apps=$state<{id:string;name:string}[]>([]);
	let picking=$state(false);let picker=$state<HTMLDialogElement>();
 $effect(()=>{if(picking&&picker&&!picker.open)picker.showModal()});
	let appQuery=$state("");
	onMount(async()=>{apps=await invoke("list_startup_apps")});
	function addApp(app:{id:string;name:string}) { if(!/^[a-zA-Z0-9_.-]+\.desktop$/.test(app.id)) return; entries=[...(entries ?? []),{command:`gtk-launch ${app.id}`,label:app.name,enabled:true}];picking=false; }
	let editSystem = $state(false);
	let originalSystem = "";
	let entries = $state<AutostartEntry[] | null>(null);

	onMount(async () => {
		entries = await invoke<AutostartEntry[]>("get_autostart_entries");
		originalSystem = JSON.stringify(entries.filter(e => isSystemStartup(e.command)));
	});

	function addEntry() {
		entries = [...(entries ?? []), { command: "", label: "", enabled: true }];
	}

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
		if (changed && !confirm("Change core desktop services? Disabling or removing them can stop locking, screenshots, wallpaper or other desktop features.")) throw new Error("System service changes were not saved.");
		await invoke("save_autostart_entries", { entries, allowSystemChanges: changed });
		originalSystem = system;
	}
	useDraft(() => (entries), save, () => entries !== null);
</script>

<ViewScaffold title="Startup" lede="Apps that open when you log in.">
	{#if entries}
		<Group
			title="Your apps"
			hint="Your apps start after login. Core desktop services are listed separately below."
			wide
		>
			{#each entries as entry, i (i)}
				{#if !isSystemStartup(entry.command)}
				<div class="row">
					<button
						class="switch"
						class:on={entry.enabled}
						role="switch"
						aria-checked={entry.enabled}
						aria-label="Enabled"
						onclick={() => (entry.enabled = !entry.enabled)}
					>
						<span class="knob"></span>
					</button>
					<div class="fields">
						<input type="text" bind:value={entry.label} placeholder="Label" class="label" title={entry.label} />
						<details><summary>Advanced command</summary><input type="text" bind:value={entry.command} placeholder="command" class="command" title={entry.command} /></details>
					</div>
					<button class="browse" onclick={() => browseFor(entry)} aria-label="Browse for program">
						<FolderOpen size={14} />
					</button>
					<button class="remove" onclick={() => removeEntry(i)}>Remove</button>
				</div>
							{/if}
			{/each}
			<button class="btn" onclick={() => picking=true}>Add an app</button>
 {#if picking}<dialog bind:this={picker} aria-label="Add a startup app" oncancel={()=>picking=false}><h3>Add a startup app</h3><input class="inp" aria-label="Search installed apps" bind:value={appQuery} placeholder="Search installed apps"/>
 <div class="picker">{#each apps.filter(a=>a.name.toLowerCase().includes(appQuery.toLowerCase())) as app (app.id)}<button class="btn" onclick={()=>addApp(app)}>{app.name}<span>Add</span></button>{/each}</div><button class="btn" onclick={()=>picking=false}>Cancel</button></dialog>{/if}
 <details><summary>Add a custom command</summary><button class="btn" onclick={addEntry}>Add custom command</button></details>


   <details class="system" open>
    <summary>Managed by BOS</summary>
    <Hint text="These services provide the bar, locking and desktop integrations. Leave them enabled unless you are replacing them." />
    <details><summary>Advanced service editing</summary><button class="btn" onclick={() => { if (confirm("Enable advanced editing of core startup services? Changing them can break desktop features.")) editSystem = true; }}>Enable advanced editing</button></details>
    {#each entries as entry, i (i)}
     {#if isSystemStartup(entry.command)}
      <div class="row managed">
       <div class="fields"><b>{entry.label || entry.command}</b><small class="state" class:off={!entry.enabled}>{entry.enabled ? "Runs at login" : "Turned off"}</small>
        {#if editSystem}<input aria-label="System service command" bind:value={entry.command} />{/if}
       </div>
       {#if editSystem}<button class="btn" onclick={() => entry.enabled = !entry.enabled}>{entry.enabled ? "Disable" : "Enable"}</button><button class="btn danger" onclick={() => removeEntry(i)}>Remove</button>{/if}
      </div>
     {/if}
    {/each}
   </details>
   <SaveButton onSave={save} />
		</Group>
	{/if}
</ViewScaffold>

<style>
 .managed{display:flex;align-items:center;gap:12px;padding:10px 4px;border-top:1px solid var(--line)}
 .managed .fields{display:flex;flex-direction:column;gap:2px;flex:1;min-width:0}
 .managed b{font-weight:400}
 .managed .state{color:var(--muted);font-size:12px}
 .managed .state.off{color:var(--red,#e0736a)}

 dialog{background:var(--surface);color:var(--fg);border:1px solid var(--line);border-radius:14px;width:min(480px,calc(100vw - 32px));}dialog::backdrop{background:#0009;}
	.picker {max-height:320px;overflow:auto;display:flex;flex-direction:column;gap:4px;margin:8px 0;}
	.picker span {margin-left:auto;}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		margin-bottom: var(--space-xs, 4px);
	}

	.fields {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.label,
	.command {
		width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.command {
		opacity: 0.65;
		font-size: var(--font-size-secondary, 12px);
	}

	input[type="text"] {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		cursor: pointer;
	}

	.browse {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
		display: flex;
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
	}


	.switch {
		width: 36px;
		height: 20px;
		flex-shrink: 0;
		border-radius: 999px;
		background-color: var(--overlay);
		padding: 2px;
		display: flex;
		align-items: center;
	}

	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}

	.knob {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}
	.system { border-top:1px solid var(--line); margin-top:12px; padding-top:12px; } summary { cursor:pointer; color:var(--muted); } small { color:var(--muted); }
</style>
