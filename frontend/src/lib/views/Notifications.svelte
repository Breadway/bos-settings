<script lang="ts">
 import { invoke } from "@tauri-apps/api/core";
 import { go } from "$lib/nav.svelte";
 import ViewScaffold from "$lib/components/ViewScaffold.svelte";
 import Group from "$lib/components/Group.svelte";
 import Row from "$lib/components/Row.svelte";
 import Hint from "$lib/components/Hint.svelte";
 let message = $state("");
 async function history() { try { await invoke("open_notification_history"); } catch(e) { message = String(e); } }
</script>
<ViewScaffold title="Notifications" lede="Notification history and the way alerts appear on your desktop.">
 <Group title="Notifications">
  <Row label="Notification history" hint="Review your notifications in the bar's notification center."><button class="btn" onclick={history}>Open history</button></Row>
  <Row label="Notification appearance" hint="The shell theme controls the notification layout and motion."><button class="btn" onclick={() => go("appearance","shelltheme")}>Choose a theme</button></Row>
  <Hint text="Do not disturb is available in the bar's control panel. Scheduling, per-app permissions, notification sounds and lock-screen previews are not yet available in Settings." />
 </Group>
 {#if message}<p role="status">{message}</p>{/if}
</ViewScaffold>
<style>p { color:var(--muted); font-size:12px; }</style>
