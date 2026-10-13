<script lang="ts">
 import {runStreamed} from "$lib/streaming";
 import LogView from "$lib/components/LogView.svelte";
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 interface Cache { name: string; bytes: number }
 interface StorageInfo { disks: {path:string; total:number; used:number; available:number}[]; package_cache:number|null; app_cache:number|null; trash:number|null; errors:string[] }
 let breakdown=$state<{label:string;bytes:number|null;error:string|null}[]>([]);
 // Measuring a full disk takes a minute; it shouldn't lock the cleanup buttons.
 let measuring=$state(false);
 async function reviewBreakdown(){measuring=true;try{breakdown=await invoke("get_storage_breakdown")}catch(e){message=String(e)}finally{measuring=false}}
 let info = $state<StorageInfo|null>(null);
 let caches = $state<Cache[]|null>(null);
 let busy = $state(false);let unused=$state<string[]|null>(null);let selected=$state<string[]>([]);let logs=$state<string[]>([]);
 async function reviewUnused(){busy=true;try{unused=await invoke('get_unused_dependencies');selected=[]}catch(e){message=String(e)}finally{busy=false}}
 async function removeUnused(){if(busy||!selected.length||!confirm(`Remove these unused dependencies?\n${selected.join(', ')}\nOnly the selected packages will be removed.`))return;busy=true;logs=[];try{const ok=await runStreamed('remove_unused_dependencies',{packages:selected},line=>logs=[...logs,line]);message=ok?'Selected dependencies removed.':'Removal failed. Review the log.';await reviewUnused()}catch(e){message=String(e)}finally{busy=false}}

 let message = $state("");
 const size = (bytes: number|null) => bytes === null ? "Unavailable" : bytes < 1024 ** 3 ? `${(bytes / 1024 ** 2).toFixed(0)} MiB` : `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
 async function refresh() { try { info = await invoke<StorageInfo>("get_storage_info"); } catch(e) { message = String(e); } }
 onMount(refresh);
 async function review() { busy = true; try { caches = await invoke<Cache[]>("get_cache_entries"); } catch(e) { message = String(e); } finally { busy = false; } }
 async function clean(target: {kind:string; name?:string}, label:string) {
  if (!confirm(`${label}? This permanently deletes these files and cannot be undone.${target.kind === "app_cache" ? " Close the app first. It will rebuild its cache when needed." : ""}`)) return;
  busy = true; message = "";
  try { await invoke("clean_storage", {target}); message = "Cleanup complete."; await refresh(); if(caches) caches = await invoke<Cache[]>("get_cache_entries"); }
  catch(e) { message = String(e); } finally { busy = false; }
 }
</script>
<ViewScaffold title="Storage" lede="See what is using space, and clear what you don't need.">
 {#if info}
  {#each info.disks as disk (disk.path)}
   {@const pct = disk.total ? disk.used / disk.total : 0}
   <div class="hero-card disk">
    <div class="grow">
     <h3>{size(disk.available)} free</h3>
     <p>{info.disks.length === 1 || disk.path === "/" ? "This computer" : disk.path} · {size(disk.used)} of {size(disk.total)} used</p>
     <div class="bar" class:full={pct > 0.85}><i style="width:{(pct * 100).toFixed(1)}%"></i></div>
     {#if pct > 0.85}<p class="warn">Getting full. Clearing the items below can help.</p>{/if}
    </div>
   </div>
  {/each}
  <Group title="Free up space">
   <Row label="Old package downloads" hint={`${size(info.package_cache)} · BOS keeps the last 3 versions of each package.`}><button class="btn small" disabled={busy} onclick={() => clean({kind:"package_cache"}, "Clean older package downloads")}>Clean</button></Row>
   <Row label="App caches" hint={`${size(info.app_cache)} · apps rebuild these when needed.`}><button class="btn small" disabled={busy} onclick={review}>Review</button></Row>
   {#if caches}
    <div class="sub">
     {#each caches.filter((c) => c.bytes >= 1024 ** 2).sort((a, b) => b.bytes - a.bytes) as cache (cache.name)}<Row label={cache.name} hint={size(cache.bytes)}><button class="btn small remove" disabled={busy} onclick={() => clean({kind:"app_cache",name:cache.name}, `Delete the ${cache.name} cache`)}>Clean</button></Row>{:else}<p>No app caches found.</p>{/each}
     <p>Close an app before cleaning its cache.</p>
    </div>
   {/if}
   <Row label="Trash" hint={size(info.trash)}><button class="btn small remove" disabled={busy || info.trash === 0} onclick={() => clean({kind:"trash"}, "Empty Trash")}>Empty</button></Row>
   <Row label="Unused packages" hint="Installed to support apps that are gone now."><button class="btn small" disabled={busy} onclick={reviewUnused}>Review</button></Row>
   {#if unused}
    <div class="sub">
     {#each unused as name (name)}<label class="pkg"><input type="checkbox" value={name} bind:group={selected}/>{name}</label>{:else}<p>Nothing unused.</p>{/each}
     {#if unused.length}<button class="btn small remove" disabled={busy||!selected.length} onclick={removeUnused}>Remove {selected.length || ""} selected</button>{/if}
    </div>
   {/if}
  </Group>
  <Group title="Where space goes">
   {#if breakdown.length}
    {#each breakdown as entry (entry.label)}<Row label={entry.label} hint={entry.error ?? undefined}><span class="num">{entry.label === "System snapshots" && entry.bytes !== null && info.disks[0] && entry.bytes > info.disks[0].used ? "Shared with the system" : size(entry.bytes)}</span></Row>{/each}
    <p>Snapshots share space with the system, so these numbers overlap. Folders only the system can read aren't counted.</p>
   {:else}
    <Row label="Measure folders" hint="Takes a moment on a full disk."><button class="btn small" disabled={measuring} onclick={reviewBreakdown}>{measuring ? "Measuring…" : "Measure"}</button></Row>
   {/if}
  </Group>
  {#if logs.length}<details class="ui-disclosure"><summary>Details</summary><LogView lines={logs}/></details>{/if}
  {#each info.errors as error}<p>{error}</p>{/each}
 {:else}<p>{message || "Measuring storage…"}</p>{/if}
 <div class="actions"><button class="btn small ghost" disabled={busy} onclick={refresh}>Refresh</button><button class="btn small ghost" onclick={() => invoke("open_storage_folder", {cache:false}).catch(e => message = String(e))}>Open your files</button></div>
 {#if message}<p role="status">{message}</p>{/if}
</ViewScaffold>
<style>
 .grow { flex:1; min-width:0; }
 .bar { height:8px; border-radius:99px; background:var(--surface-2); overflow:hidden; margin-top:12px; }
 .bar i { display:block; height:100%; background:var(--accent); border-radius:99px; }
 .bar.full i { background:#d6b25e; }
 .warn { color:#d6b25e !important; margin-top:8px !important; }
 .sub { margin:0 0 6px 14px; padding-left:12px; border-left:2px solid var(--line); }
 .pkg { display:flex; gap:8px; align-items:center; font-size:12.5px; padding:4px 0; }
 .num { color:var(--muted); font-size:12.5px; }
 .actions { display:flex; gap:8px; flex-wrap:wrap; }
 p { color:var(--muted); font-size:12px; margin:4px 2px 8px; }
</style>
