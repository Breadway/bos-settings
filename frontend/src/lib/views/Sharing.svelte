<script lang="ts">
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import { go } from "$lib/nav.svelte";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import { runStreamed } from "$lib/streaming";
 import SwitchField from "$lib/components/SwitchField.svelte";
 interface Status { hostname:string; ssh_installed:boolean; ssh_running:boolean; localsend_installed:boolean; phone_link_installed:boolean }
 let status = $state<Status|null>(null); let busy = $state(false); let message = $state("");
 let hostname = $state("");
 async function refresh() { try { status = await invoke<Status>("get_sharing_status"); hostname = status.hostname; } catch(e) { message = String(e); } }
 async function rename() {
  const name = hostname.trim();
  if (!name || busy) return;
  busy = true; message = "";
  try { await invoke("set_hostname", { name }); message = `This computer is now called ${name}.`; await refresh(); }
  catch(e) { message = `Couldn't rename it: ${e}`; }
  finally { busy = false; }
 }
 async function installPhoneLink() {
  busy = true; message = "";
  try { const ok = await runStreamed("pacman_install", { packages: ["kdeconnect"] }, () => {}); if (ok === false) message = "KDE Connect couldn't be installed."; await refresh(); }
  catch(e) { message = String(e); }
  finally { busy = false; }
 }
 onMount(refresh);
 async function ssh(enabled:boolean) {
  if(busy) return;
  if(enabled && !confirm("Enable remote login? Other computers can try to log in to this computer using SSH. Firewall rules still apply.")) return;
  busy = true; message = "";
  try { await invoke("set_remote_login", {enabled}); await refresh(); } catch(e) { message = String(e); } finally { busy = false; }
 }
 async function open(app:string) { try { await invoke("open_sharing_app", {app}); } catch(e) { message = String(e); } }
</script>
<ViewScaffold title="Sharing" lede="Let nearby devices send you things, or reach this computer.">
 {#if status}
  <Group title="Nearby devices">
   <Row label="Receive files" hint={status.localsend_installed ? "Phones and laptops on this network can send you files with LocalSend." : "LocalSend lets phones and laptops nearby send you files."}>
    <button class="btn small" onclick={() => status?.localsend_installed ? open("local_send") : go("getapps")}>{status.localsend_installed ? "Open" : "Get LocalSend"}</button>
   </Row>
   <Row label="Phone link" hint="Notifications, clipboard and files with your phone, through KDE Connect.">
    {#if status.phone_link_installed}<button class="btn small" onclick={() => open("phone_link")}>Open</button>
    {:else}<button class="btn small" disabled={busy} onclick={installPhoneLink}>{busy ? "Installing…" : "Install"}</button>{/if}
   </Row>
  </Group>
  <Group title="Remote login">
   {#if status.ssh_installed}<SwitchField label="Remote login (SSH)" hint="Lets you open a terminal on this computer from another one. Asks for your password." bind:value={() => status!.ssh_running, ssh} />{:else}<p>The SSH server isn't installed, so this computer can't be reached remotely.</p>{/if}
   <Row label="Firewall" hint="Choose what can connect in."><button class="btn small ghost" onclick={() => go("privacy")}>Open</button></Row>
  </Group>
  <Group title="This computer">
   <Row label="Name on the network" hint={`Others see it as ${status.hostname}.local. Needs your password.`}>
    <input class="ui-input name" bind:value={hostname} onkeydown={(e) => e.key === "Enter" && rename()} />
    <button class="btn small" disabled={busy || !hostname.trim() || hostname.trim() === status.hostname} onclick={rename}>Rename</button>
   </Row>
  </Group>
 {:else}<p>{message || "Checking sharing services…"}</p>{/if}
 {#if message}<p role="status">{message}</p>{/if}
</ViewScaffold>
<style>p { font-size:12.5px; color:var(--muted); margin:4px 2px 8px; } .name { width:150px; }</style>
