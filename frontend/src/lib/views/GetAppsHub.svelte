<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
 import {invoke} from '@tauri-apps/api/core';
 import {runStreamed} from '$lib/streaming';
 import ViewScaffold from '$lib/components/ViewScaffold.svelte';
 import Group from '$lib/components/Group.svelte';
 import LogView from '$lib/components/LogView.svelte';
 import Optional from './Optional.svelte';
 import Packages from './Packages.svelte';
 import Embed from '$lib/components/Embed.svelte';
 interface App {id:string;name:string;version:string;description:string;source:'repository'|'aur'|'flathub';installed:boolean;votes:number|null}
 const categories=[['Communication','chat'],['Media','video'],['Games','game'],['Creativity','graphics'],['Development','editor']] as const;
 let source=$state('all');
 let query=$state('');let apps=$state<App[]>([]);let errors=$state<string[]>([]);let searched=$state(false);let busy=$state(false);let installing=$state('');let logs=$state<string[]>([]);let epoch=0;let visible=$derived(apps.filter(app=>source==='all'||app.source===source));
 async function search(){const n=++epoch;busy=true;errors=[];try{const result=await invoke<{apps:App[];errors:string[]}>('search_apps',{query});if(n===epoch){apps=result.apps;errors=result.errors;searched=true}}catch(e){errors=[String(e)]}finally{if(n===epoch)busy=false}}
 async function install(app:App){if(installing)return;
 if(app.source==='aur' && !(await confirmAction({title:'Install from the AUR?',message:'AUR packages contain community build scripts. Continue to interactive PKGBUILD review in a terminal?',confirmLabel:'Continue'})))return;
 installing=app.id;logs=[];try{const ok=await runStreamed('install_store_app',{id:app.id,source:app.source},line=>logs=[...logs,line]);if(!ok)errors=['Installation failed. Review the log.'];else if(app.source==='aur')errors=['Review and finish the install in the opened terminal, then search again to refresh installed status.'];else await search()}catch(e){errors=[String(e)]}finally{installing=''}}
</script>
<ViewScaffold title="Get apps" lede="Apps from Flathub, the BOS repositories and the AUR, in one place.">
 <form onsubmit={e=>{e.preventDefault();void search()}} class="search"><input class="inp" aria-label="Search apps" bind:value={query} placeholder="Search for an app"/><button class="btn primary" disabled={busy||!query.trim()}>{busy?'Searching…':'Search'}</button></form>
 <div class="categories">{#each categories as [name,term]}<button class="btn" disabled={busy} onclick={()=>{query=term;void search()}}>{name}</button>{/each}</div>
 <div class="categories" aria-label="App sources">{#each [['all','All'],['repository','BOS repositories'],['flathub','Flathub'],['aur','AUR']] as [id,label]}<button class="btn" class:primary={source===id} aria-pressed={source===id} onclick={()=>source=id}>{label}</button>{/each}</div>
 {#if searched}<Group title="Results">
 {#each visible as app (`${app.source}:${app.id}`)}<div class="app-row"><div class="icon">{app.name.slice(0,1).toUpperCase()}</div><div class="label"><b>{app.name}</b> <span class="tag">{app.source==='aur'?'AUR · Community':app.source==='flathub'?'Flathub':'BOS repositories'}</span><small>{app.description}{app.votes!==null?` · ${app.votes} votes`:''}</small></div><button class="btn" disabled={app.installed||!!installing} onclick={()=>install(app)}>{app.installed?'Installed':installing===app.id?'Installing…':'Install'}</button></div>{/each}
 {#if !visible.length}<p>No matching apps found.</p>{/if}
 </Group>{/if}
 {#each errors as error}<p role="status">{error}</p>{/each}{#if logs.length}<LogView lines={logs}/>{/if}
 <details open={!searched}><summary>Recommended apps</summary><Embed><Optional/></Embed></details>
 <details><summary>Installed packages and advanced management</summary><Embed><Packages/></Embed></details>
</ViewScaffold>
<style>.categories{display:flex;gap:6px;flex-wrap:wrap;}.search{display:flex;gap:10px;padding:12px 14px;border-radius:14px;background:var(--surface);border:1px solid var(--line)}.search input{flex:1;min-width:0}.app-row{display:flex;align-items:center;gap:12px;padding:11px 0;border-top:1px solid var(--line)}.icon{width:34px;height:34px;display:grid;place-items:center;border-radius:10px;background:var(--surface-2);color:var(--accent)}.label{flex:1;min-width:0}.label b{font-weight:400}.label small{display:block;color:var(--muted);font-size:12px}.tag{font-size:10px;background:var(--surface-2);border-radius:5px;padding:4px 6px;color:var(--muted)}@media(max-width:560px){.app-row{flex-wrap:wrap}.label{min-width:150px}}</style>
