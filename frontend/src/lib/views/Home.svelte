<script lang="ts">
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import { greetingWithName } from "$lib/greeting";
 import { go } from "$lib/nav.svelte";
 import type {NotificationPreferences} from "$lib/notificationPreferences";
 import type {RadioState} from "$lib/home.svelte";
 import Bell from "@lucide/svelte/icons/bell";
 import Zap from "@lucide/svelte/icons/zap";
 import Plane from "@lucide/svelte/icons/plane";
 import { home } from "$lib/home.svelte";
 import { updateCounts } from "$lib/updateCount";
 import { changes, recordChange, undoChange } from "$lib/changes.svelte";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import Hint from "$lib/components/Hint.svelte";
 import Wifi from "@lucide/svelte/icons/wifi";
 import Bluetooth from "@lucide/svelte/icons/bluetooth";
 import Moon from "@lucide/svelte/icons/moon";



 const hour = new Date().getHours();
 let greeting = $derived(greetingWithName(hour, home.account?.display_name?.trim() || home.account?.username?.trim() || null));
 let busy = $state(false);
 let pending = $derived(updateCounts(home.updates).total);
 const size=(bytes:number)=>`${(bytes/1024**3).toFixed(1)} GiB`;
 let dnd=$derived(Boolean(home.notifications?.effective_dnd ?? home.notifications?.dnd));
 let disk=$derived(home.storage?.disks[0]);
 let connected=$derived(home.devices?.filter(d=>d.connected) ?? []);
 let timeLeft=$derived(home.power?.battery.find(([k])=>k==="Time remaining")?.[1]);
 const ERROR_LABELS:Record<string,string>={wifi:"Wi-Fi",bluetooth:"Bluetooth",night:"Night light",power:"Battery",updates:"Updates",backup:"Backup",account:"Account",notifications:"Notifications",modes:"Power mode",radios:"Airplane mode",storage:"Storage",devices:"Bluetooth devices",automatic:"Automatic power mode"};
 let untilFull=$derived(home.power?.battery.find(([k])=>k==="Time until full")?.[1]);
 let battState=$derived(home.power?.battery.find(([k])=>k==="Status")?.[1]);
 // Short battery phrase: time left on battery, time to full while charging, else the state.
 let battNote=$derived(timeLeft ? `about ${timeLeft} left` : untilFull ? `charging, full in ${untilFull}` : battState ? battState.toLowerCase() : "");
 let cache=$derived(home.storage ? [home.storage.package_cache,home.storage.app_cache,home.storage.trash].reduce<number>((s,v)=>s+(v??0),0) : null);
 let charge = $derived(home.power?.battery.find(([k]) => k === "Charge")?.[1]);
 async function read<T>(key:keyof typeof home, command:string) {
  try { const result = await invoke<T>(command); Object.assign(home,{[key]:result}); delete home.errors[key]; }
  catch(e) { home.errors[key] = String(e); }
 }
 async function refresh() {
  await Promise.all([read("wifi","get_network_info"),read("bluetooth","get_adapter_powered"),read("night","get_nightlight"),read("power","get_power_info"),read("updates","get_updates_status"),read("backup","get_backup_config"),read("account","get_current_account"),read("notifications","get_notification_preferences"),read("modes","get_power_modes"),read("radios","get_radio_states"),read("storage","get_storage_info"),read("devices","get_paired_devices"),read("automatic","get_automatic_power_mode")]);
  home.lastCheck = Date.now();
 }
 onMount(() => { if(Date.now() - home.lastCheck > 60000) void refresh(); });
 async function toggle(kind:"wifi"|"bluetooth"|"night"|"dnd"|"mode"|"plane") {
  if(busy) return;
  busy = true;
  try {
   if(kind === "dnd" && home.notifications) {
    const before=structuredClone($state.snapshot(home.notifications));const wasOn=Boolean(before.effective_dnd ?? before.dnd);
    const set=async (prefs:NotificationPreferences)=>{await invoke('save_notification_preferences',{preferences:prefs});home.notifications=await invoke("get_notification_preferences")};
    await recordChange(`Do not disturb ${wasOn?'off':'on'}`,()=>set({...before,dnd:!wasOn,schedule:wasOn?false:before.schedule}),async()=>{const latest=await invoke<NotificationPreferences>('get_notification_preferences');await set({...latest,dnd:before.dnd,schedule:before.schedule});});
   } else if(kind === "mode" && home.modes?.profiles.length) {
    const before=home.modes.current;const next=home.modes.profiles[(home.modes.profiles.indexOf(before)+1)%home.modes.profiles.length];
    const set=async(profile:string)=>{await invoke('set_power_mode',{profile});if(home.modes)home.modes.current=profile};
    await recordChange(`Power mode: ${next}`,()=>set(next),()=>set(before));
   } else if(kind === "plane" && home.radios?.length) {
    const before=$state.snapshot(home.radios);const enabled=!before.every(r=>r.blocked);
    await recordChange(`Airplane mode ${enabled?'on':'off'}`,async()=>{home.radios=await invoke('set_airplane_mode',{enabled})},async()=>{await invoke('restore_radio_states',{radios:before});home.radios=await invoke('get_radio_states')});
   } else if(kind === "wifi" && home.wifi) {
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
<ViewScaffold title={greeting} lede={`${home.wifi?.ssid ? `On ${home.wifi.ssid}` : "Your desktop at a glance"}${charge ? ` · battery ${charge}` : ""}${battNote ? `, ${battNote}` : ""}${connected.length ? ` · ${connected.map(d=>d.name).join(", ")} connected` : ""}.`}>
 <div class="quick">
  <button class="tile" class:on={home.wifi?.radio_enabled} disabled={busy || !home.wifi} onclick={() => toggle("wifi")} aria-pressed={home.wifi?.radio_enabled ?? false}><Wifi size={21} /><b>Wi-Fi</b><small>{home.wifi ? home.wifi.radio_enabled ? home.wifi.ssid || "On" : "Off" : home.errors.wifi ? "Unavailable" : "Checking…"}</small></button>
  <button class="tile" class:on={home.bluetooth} disabled={busy || home.bluetooth === null} onclick={() => toggle("bluetooth")} aria-pressed={home.bluetooth ?? false}><Bluetooth size={21} /><b>Bluetooth</b><small>{home.bluetooth === null ? home.errors.bluetooth ? "Unavailable" : "No adapter" : home.bluetooth ? connected.map(d=>d.name).join(", ") || "On" : "Off"}</small></button>
  <button class="tile" class:on={!!home.night?.enabled} disabled={busy || !home.night || !home.night.installed} onclick={() => toggle("night")} aria-pressed={!!home.night?.enabled}><Moon size={21} /><b>Night light</b><small>{home.night ? home.night.installed ? home.night.enabled ? "On" : "Off" : "Not installed" : "Checking…"}</small></button>
  <button class="tile" class:on={dnd} disabled={busy||!home.notifications} onclick={()=>toggle('dnd')} aria-pressed={dnd} title={home.notifications ? undefined : "The running bar doesn't offer notification settings yet."}><Bell size={21}/><b>Do not disturb</b><small>{home.notifications ? dnd ? 'On' : home.notifications.schedule ? 'Scheduled' : 'Off' : 'Not available'}</small></button>
  <button class="tile" class:on={!!home.modes?.current} disabled={busy||home.automatic||(!home.modes?.profiles.length&&!home.modes?.tlp_setup_available)} onclick={()=>home.modes?.profiles.length ? toggle('mode') : go('power')}><Zap size={21}/><b>Power mode{home.automatic ? " · Auto" : ""}</b><small>{({'power-saver':'Battery saver',balanced:'Balanced',performance:'Performance'} as Record<string,string>)[home.modes?.current ?? ''] ?? (home.modes?.tlp_setup_available ? 'Set up in Power' : 'Not available')}</small></button>
  <button class="tile" class:on={!!home.radios?.length&&home.radios.every(r=>r.blocked)} disabled={busy||!home.radios?.length} onclick={()=>toggle('plane')} aria-pressed={!!home.radios?.length&&home.radios.every(r=>r.blocked)}><Plane size={21}/><b>Airplane mode</b><small>{home.radios?.length ? home.radios.every(r=>r.blocked) ? 'Radios off' : 'Off' : 'No radios found'}</small></button>
 </div>
 <Group title="Needs your attention">
  <Row label={cache===null ? "Review disk space" : `${size(cache)} in caches and Trash`} hint="Package cleanup keeps the newest versions. Review totals before deleting."><button class="btn" onclick={()=>go("storage")}>Manage</button></Row>
  <Row label={home.updates ? pending ? `${pending} updates ready` : "No package updates found" : "Updates"} hint={home.updates?.pacman_error || home.updates?.bakery_error ? "Some update checks failed. Open Updates for details." : "System packages, bread apps and firmware."}><button class="btn primary" onclick={() => go("updates")}>Review</button></Row>
  <Row label={home.backup ? home.backup.repo ? "Your file backup" : "Set up a file backup" : "Backup & restore"} hint="System snapshots protect the system. A separate backup protects your personal files."><button class="btn" onclick={() => go("restore","backup")}>{home.backup?.repo ? "Review" : "Set up"}</button></Row>
 </Group>
 <div class="summaries">
 {#if home.power}<div class="summary"><div class="ring">{charge || "AC"}</div><div><b>Battery</b><small>{timeLeft ? `${timeLeft} left` : untilFull ? `Full in ${untilFull}` : battState ?? home.power.power_source} · {home.power.battery.find(([k])=>k==='Battery health')?.[1] ?? 'Health unavailable'}</small></div></div>{/if}
 {#each connected as device (device.address)}<div class="summary"><Bluetooth size={24}/><div><b>{device.name}</b><small>Connected{device.battery===null ? '' : ` · ${device.battery}% battery`}</small></div></div>{/each}
 {#if disk}<div class="summary"><div><b>Storage</b><small>{size(disk.used)} of {size(disk.total)} used</small><progress aria-label="Disk space used" value={disk.used} max={disk.total}></progress></div></div>{/if}
 </div>
 {#if changes.recent.length}
  <Group title="Recently changed">
   {#each changes.recent as change (change.id)}<Row label={change.label} hint={change.at.toLocaleTimeString([], {hour:"2-digit",minute:"2-digit"})}>{#if change.undo}<button class="btn" disabled={changes.busy} onclick={() => undoChange(change)}>Undo</button>{:else}<small>Undone</small>{/if}</Row>{/each}
  </Group>
 {/if}
 {#if Object.keys(home.errors).length}
  <details class="status-errors"><summary>Some status couldn't be read ({Object.keys(home.errors).length})</summary>
   {#each Object.entries(home.errors) as [key,error]}<Hint text={`${ERROR_LABELS[key] ?? key}: ${error}`} />{/each}
   <button class="btn refresh" onclick={refresh}>Try again</button>
  </details>
 {/if}
</ViewScaffold>
<style>
 .summaries{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:10px}.summary{display:flex;align-items:center;gap:12px;padding:14px;border:1px solid var(--line);background:var(--surface);border-radius:14px}.summary b{font-weight:400}.summary small{display:block;color:var(--muted);font-size:12px}.summary progress{width:100%;accent-color:var(--accent)}.ring{width:48px;height:48px;border:4px solid var(--accent);border-radius:50%;display:grid;place-items:center;flex:none}
 .quick { display:grid; grid-template-columns:repeat(auto-fit,minmax(170px,1fr)); gap:10px; }
 .tile { display:flex; flex-direction:column; gap:10px; min-width:0; padding:16px; background:var(--surface); border:1px solid var(--line); color:var(--fg); border-radius:14px; text-align:left; cursor:pointer; }
 .tile b { font-weight:400; } .tile small { color:var(--muted); font-size:12px; } .tile.on { background:var(--accent-soft); border-color:color-mix(in srgb,var(--accent) 55%,transparent); }
 .tile.on :global(svg) { color:var(--accent); } .tile:disabled { opacity:.6; cursor:default; } .refresh { align-self:flex-start; margin-top:8px; } .status-errors { color:var(--muted); font-size:12.5px; } .status-errors summary { cursor:pointer; padding:4px 0; }
</style>
