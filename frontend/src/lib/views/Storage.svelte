<script lang="ts">
 import {runStreamed} from "$lib/streaming";
 import LogView from "$lib/components/LogView.svelte";
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import Hint from "$lib/components/Hint.svelte";
 interface Cache { name: string; bytes: number }
 interface StorageInfo { disks: {path:string; total:number; used:number; available:number}[]; package_cache:number|null; app_cache:number|null; trash:number|null; errors:string[] }
 let breakdown=$state<{label:string;bytes:number|null;error:string|null}[]>([]);
 async function reviewBreakdown(){busy=true;try{breakdown=await invoke("get_storage_breakdown")}catch(e){message=String(e)}finally{busy=false}}
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
<ViewScaffold title="Storage" lede="See what is using space and review what you can remove.">
 {#if info}
  <Group title="Disk space">
   {#each info.disks as disk (disk.path)}
    <Row label={disk.path}><span>{size(disk.used)} of {size(disk.total)} used</span></Row>
    <progress aria-label={`Space used on ${disk.path}`} value={disk.used} max={disk.total}></progress>
    <Hint text={`${size(disk.available)} available`} />
   {/each}
  </Group>
  <Group title="Where space is used" hint="Directory usage, measured when requested. Your files include app caches and Trash. Btrfs snapshots share data with the system, so these figures overlap and must not be added or treated as reclaimable space."><button class="btn" disabled={busy} onclick={reviewBreakdown}>Measure directories</button>{#each breakdown as entry (entry.label)}<Row label={entry.label} hint={entry.error ?? undefined}><span>{size(entry.bytes)}</span></Row>{/each}</Group>
  <Group title="Free up space">
   <Row label="Package downloads" hint={`${size(info.package_cache)} cached. Cleanup keeps the latest 3 versions of each package.`}><button class="btn" disabled={busy} onclick={() => clean({kind:"package_cache"}, "Clean older package downloads")}>Clean older versions</button></Row>
   <Row label="App caches" hint={`${size(info.app_cache)}. Review each app before deleting.`}><button class="btn" disabled={busy} onclick={review}>Review</button></Row>
   <Row label="Trash" hint={size(info.trash)}><button class="btn danger" disabled={busy} onclick={() => clean({kind:"trash"}, "Empty Trash")}>Empty</button></Row>
  </Group>
  {#if caches}
   <Group title="App caches" hint="Close an app before cleaning its cache. Downloaded files stored outside the cache are kept.">
    {#each caches as cache (cache.name)}<Row label={cache.name} hint={size(cache.bytes)}><button class="btn danger" disabled={busy} onclick={() => clean({kind:"app_cache",name:cache.name}, `Delete the ${cache.name} cache`)}>Clean</button></Row>{:else}<Hint text="No app cache directories found." />{/each}
   </Group>
  {/if}
  <Group title="Unused dependencies" hint="Packages installed as dependencies that no installed package requires. Review before removing them."><button class="btn" disabled={busy} onclick={reviewUnused}>Review</button>{#if unused}{#each unused as name (name)}<Row label={name}><input type="checkbox" aria-label={`Remove ${name}`} value={name} bind:group={selected}/></Row>{:else}<Hint text="No unused dependencies found."/>{/each}<button class="btn danger" disabled={busy||!selected.length} onclick={removeUnused}>Remove selected</button>{/if}</Group>
  <LogView lines={logs}/>
  {#each info.errors as error}<Hint text={error} />{/each}
 {:else}<Hint text={message || "Measuring storage…"} />{/if}
 <div class="actions"><button class="btn" disabled={busy} onclick={refresh}>Refresh</button><button class="btn" onclick={() => invoke("open_storage_folder", {cache:false}).catch(e => message = String(e))}>Open your files</button></div>
 {#if message}<p role="status">{message}</p>{/if}
</ViewScaffold>
<style>progress { width:100%; accent-color:var(--accent); margin:0 0 8px; } .actions { display:flex; gap:8px; flex-wrap:wrap; } p { color:var(--muted); font-size:12px; }</style>
