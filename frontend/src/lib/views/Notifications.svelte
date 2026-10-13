<script lang="ts">
 import {onMount} from 'svelte';
 import {invoke} from '@tauri-apps/api/core';
 import type {NotificationPreferences} from '$lib/notificationPreferences';
 import {useDraft} from '$lib/drafts.svelte';
 import {go} from '$lib/nav.svelte';
 import ViewScaffold from '$lib/components/ViewScaffold.svelte';
 import Group from '$lib/components/Group.svelte';
 import Row from '$lib/components/Row.svelte';
 import SwitchField from '$lib/components/SwitchField.svelte';
 import SelectField from '$lib/components/SelectField.svelte';

 let prefs=$state<NotificationPreferences|null>(null);let error=$state('');
 onMount(async()=>{try{prefs=await invoke('get_notification_preferences')}catch(e){error=String(e)}});
 async function save(){await invoke('save_notification_preferences',{preferences:prefs})}
 useDraft(()=>prefs,save,()=>prefs!==null);
 async function history(){try{await invoke('open_notification_history')}catch(e){error=String(e)}}
</script>
<ViewScaffold title="Notifications" lede="Choose what can interrupt you, and when.">
 {#if prefs}
 <Group title="Notifications">
 <SwitchField label="Do not disturb" hint="Only critical alerts get through." bind:value={prefs.dnd}/>
 <SwitchField label="Turn on automatically" hint="Use a nightly schedule." bind:value={prefs.schedule}/>
 {#if prefs.schedule}<Row label="Quiet hours"><input class="inp" type="time" aria-label="Quiet hours start" bind:value={prefs.start}/><span>to</span><input class="inp" type="time" aria-label="Quiet hours end" bind:value={prefs.end}/></Row>{/if}
 <SwitchField label="Show on the lock screen" bind:value={prefs.lock_screen}/>
 <SwitchField disabled={prefs.sound_available===false} hint={prefs.sound_available===false ? "Install libcanberra to enable notification sounds." : undefined} label="Sound" bind:value={prefs.sound}/>
 </Group>
 <Group title="Apps">
 {#each Object.keys(prefs.apps).sort() as app (app)}<SelectField label={app} bind:value={prefs.apps[app]} options={['all','banners','history','off']} labels={{all:'Banners and sound',banners:'Banners',history:'Only in history',off:'Off'}}/>{/each}
 {#if !Object.keys(prefs.apps).length}<p>Apps appear here after sending a notification.</p>{/if}
 </Group> {/if}
 <Group title="History and appearance"><Row label="Notification history"><button class="btn" onclick={history}>Open history</button></Row><Row label="Notification appearance"><button class="btn" onclick={()=>go('appearance','shelltheme')}>Choose a theme</button></Row></Group>
 {#if error}
  {#if error.includes('notification settings API') || error.includes('org.freedesktop.Notifications')}
   <div class="unsupported" role="status"><b>Notification settings aren't available yet</b><span>The bar that shows your notifications doesn't offer these settings in the version you're running. Notification history above still works.</span><details><summary>Details</summary><code>{error}</code></details></div>
  {:else}<p role="alert">{error}</p>{/if}
 {/if}
</ViewScaffold>

<style>
 .unsupported{display:flex;flex-direction:column;gap:4px;padding:12px 14px;border:1px solid var(--line);border-radius:var(--radius,14px);background:var(--surface);font-size:13px}.unsupported span{color:var(--muted)}.unsupported code{font-size:11.5px;color:var(--muted);word-break:break-word}
</style>
