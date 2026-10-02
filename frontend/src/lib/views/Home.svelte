<script lang="ts">
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import { go } from "$lib/nav.svelte";
 import { home } from "$lib/home.svelte";
 import { changes, recordChange, undoChange } from "$lib/changes.svelte";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import Hint from "$lib/components/Hint.svelte";
 import Wifi from "@lucide/svelte/icons/wifi";
 import Bluetooth from "@lucide/svelte/icons/bluetooth";
 import Moon from "@lucide/svelte/icons/moon";
 import Battery from "@lucide/svelte/icons/battery";
 import Volume from "@lucide/svelte/icons/volume-2";
 import HardDrive from "@lucide/svelte/icons/hard-drive";
 const hour = new Date().getHours();
 const greeting = hour < 5 ? "Still up?" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
 let busy = $state(false);
 let pending = $derived(home.updates ? home.updates.pacman.length + home.updates.bakery.length : 0);
 let charge = $derived(home.power?.battery.find(([k]) => k === "Charge")?.[1]);
 async function read<T>(key:keyof typeof home, command:string) {
  try { const result = await invoke<T>(command); Object.assign(home,{[key]:result}); delete home.errors[key]; }
  catch(e) { home.errors[key] = String(e); }
 }
 async function refresh() {
  await Promise.all([read("wifi","get_network_info"),read("bluetooth","get_adapter_powered"),read("night","get_nightlight"),read("power","get_power_info"),read("updates","get_updates_status"),read("backup","get_backup_config")]);
  home.lastCheck = Date.now();
 }
 onMount(() => { if(Date.now() - home.lastCheck > 60000) void refresh(); });
 async function toggle(kind:"wifi"|"bluetooth"|"night") {
  if(busy) return;
  busy = true;
  try {
   if(kind === "wifi" && home.wifi) {
    const before = home.wifi.radio_enabled;
    const set = async (enabled:boolean) => { await invoke("set_wifi_radio",{enabled}); if(home.wifi) home.wifi.radio_enabled = enabled; };
    await recordChange(`Wi-Fi turned ${before ? "off" : "on"}`, () => set(!before), () => set(before));
   } else if(kind === "bluetooth" && home.bluetooth !== null) {
    const before = home.bluetooth;
    const set = async (on:boolean) => { await invoke("set_adapter_powered",{on}); home.bluetooth = on; };
    await recordChange(`Bluetooth turned ${before ? "off" : "on"}`, () => set(!before), () => set(before));
   } else if(kind === "night" && home.night) {
    const before = {...home.night};
    const set = async (state:Record<string,unknown>) => { const result = await invoke<Record<string,unknown>>("set_nightlight",state); home.night = result; if(result.error) throw new Error(String(result.error)); };
    await recordChange(`Night light turned ${before.enabled ? "off" : "on"}`, () => set({...before,enabled:!before.enabled}), async () => { const latest = await invoke<Record<string,unknown>>("get_nightlight"); await set({...latest,enabled:before.enabled}); });
   }
  } catch(e) { changes.message = String(e); changes.undo = null; }
  finally { busy = false; }
 }
</script>
<ViewScaffold title={greeting} lede="Your desktop at a glance.">
 <div class="quick">
  <button class="tile" class:on={home.wifi?.radio_enabled} disabled={busy || !home.wifi} onclick={() => toggle("wifi")} aria-pressed={home.wifi?.radio_enabled ?? false}><Wifi size={21} /><b>Wi-Fi</b><small>{home.wifi ? home.wifi.radio_enabled ? "On" : "Off" : home.errors.wifi ? "Unavailable" : "Checking…"}</small></button>
  <button class="tile" class:on={home.bluetooth} disabled={busy || home.bluetooth === null} onclick={() => toggle("bluetooth")} aria-pressed={home.bluetooth ?? false}><Bluetooth size={21} /><b>Bluetooth</b><small>{home.bluetooth === null ? home.errors.bluetooth ? "Unavailable" : "No adapter" : home.bluetooth ? "On" : "Off"}</small></button>
  <button class="tile" class:on={!!home.night?.enabled} disabled={busy || !home.night || !home.night.installed} onclick={() => toggle("night")} aria-pressed={!!home.night?.enabled}><Moon size={21} /><b>Night light</b><small>{home.night ? home.night.installed ? home.night.enabled ? "On" : "Off" : "Not installed" : "Checking…"}</small></button>
  <button class="tile" onclick={() => go("power")}><Battery size={21} /><b>Battery</b><small>{charge ?? home.power?.power_source ?? (home.errors.power ? "Unavailable" : "Checking…")}</small></button>
  <button class="tile" onclick={() => go("sound")}><Volume size={21} /><b>Sound</b><small>Volume and devices</small></button>
  <button class="tile" onclick={() => go("storage")}><HardDrive size={21} /><b>Storage</b><small>Review disk space</small></button>
 </div>
 <Group title="Needs your attention">
  <Row label={home.updates ? pending ? `${pending} updates ready` : "No package updates found" : "Updates"} hint={home.updates?.pacman_error || home.updates?.bakery_error ? "Some update checks failed. Open Updates for details." : "System packages, bread apps and firmware."}><button class="btn primary" onclick={() => go("updates")}>Review</button></Row>
  <Row label={home.backup ? home.backup.repo ? "Your file backup" : "Set up a file backup" : "Backup & restore"} hint="System snapshots protect the system. A separate backup protects your personal files."><button class="btn" onclick={() => go("restore","backup")}>{home.backup?.repo ? "Review" : "Set up"}</button></Row>
 </Group>
 {#if changes.recent.length}
  <Group title="Recently changed">
   {#each changes.recent as change (change.id)}<Row label={change.label} hint={change.at.toLocaleTimeString([], {hour:"2-digit",minute:"2-digit"})}>{#if change.undo}<button class="btn" disabled={changes.busy} onclick={() => undoChange(change)}>Undo</button>{:else}<small>Undone</small>{/if}</Row>{/each}
  </Group>
 {/if}
 {#each Object.entries(home.errors) as [key,error]}<Hint text={`${key}: ${error}`} />{/each}
 <button class="btn refresh" onclick={refresh}>Refresh status</button>
</ViewScaffold>
<style>
 .quick { display:grid; grid-template-columns:repeat(auto-fit,minmax(170px,1fr)); gap:10px; }
 .tile { display:flex; flex-direction:column; gap:10px; min-width:0; padding:16px; background:var(--surface); border:1px solid var(--line); color:var(--fg); border-radius:14px; text-align:left; cursor:pointer; }
 .tile b { font-weight:400; } .tile small { color:var(--muted); font-size:12px; } .tile.on { background:var(--accent-soft); border-color:color-mix(in srgb,var(--accent) 55%,transparent); }
 .tile.on :global(svg) { color:var(--accent); } .tile:disabled { opacity:.6; cursor:default; } .refresh { align-self:flex-start; }
</style>
