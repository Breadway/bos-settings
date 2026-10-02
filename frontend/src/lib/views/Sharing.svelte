<script lang="ts">
 import { onMount } from "svelte";
 import { invoke } from "@tauri-apps/api/core";
 import { go } from "$lib/nav.svelte";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import Hint from "$lib/components/Hint.svelte";
 import SwitchField from "$lib/components/SwitchField.svelte";
 interface Status { hostname:string; ssh_installed:boolean; ssh_running:boolean; localsend_installed:boolean; phone_link_installed:boolean }
 let status = $state<Status|null>(null); let busy = $state(false); let message = $state("");
 async function refresh() { try { status = await invoke<Status>("get_sharing_status"); } catch(e) { message = String(e); } }
 onMount(refresh);
 async function ssh(enabled:boolean) {
  if(busy) return;
  if(enabled && !confirm("Enable remote login? Other computers can try to log in to this computer using SSH. Firewall rules still apply.")) return;
  busy = true; message = "";
  try { await invoke("set_remote_login", {enabled}); await refresh(); } catch(e) { message = String(e); } finally { busy = false; }
 }
 async function open(app:string) { try { await invoke("open_sharing_app", {app}); } catch(e) { message = String(e); } }
</script>
<ViewScaffold title="Sharing" lede="Send files, connect your phone, and control remote login.">
 {#if status}
  <Group title="Nearby devices">
   <Row label="Receive files" hint={status.localsend_installed ? "Open LocalSend to send or receive files on your local network." : "Install LocalSend to share files with nearby devices."}>
    <button class="btn" onclick={() => status?.localsend_installed ? open("local_send") : go("getapps","aur")}>{status.localsend_installed ? "Open LocalSend" : "Get an app"}</button>
   </Row>
   <Row label="Phone link" hint={status.phone_link_installed ? "Pair devices and choose permissions in KDE Connect." : "KDE Connect is not installed."}>
    <button class="btn" disabled={!status.phone_link_installed} onclick={() => open("phone_link")}>Open KDE Connect</button>
   </Row>
  </Group>
  <Group title="Remote login">
   {#if status.ssh_installed}<SwitchField label="Remote login (SSH)" hint="Allow SSH logins now and after restarting. Requires authentication." bind:value={() => status!.ssh_running, ssh} />{:else}<Hint text="OpenSSH server is not installed." />{/if}
   <button class="btn" onclick={() => go("privacy")}>Review firewall rules</button>
  </Group>
  <Group title="This computer"><Row label="Name on the network"><span>{status.hostname}</span><button class="btn" onclick={() => go("about")}>Change</button></Row></Group>
 {:else}<Hint text={message || "Checking sharing services…"} />{/if}
 {#if message}<p role="status">{message}</p>{/if}
</ViewScaffold>
<style>p { font-size:12px; color:var(--muted); }</style>
