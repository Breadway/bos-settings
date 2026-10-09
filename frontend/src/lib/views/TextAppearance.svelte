<script lang="ts">
import {onMount} from 'svelte';import {invoke} from '@tauri-apps/api/core';import type {DesktopPreferences} from '$lib/desktopPreferences';import {useDraft} from '$lib/drafts.svelte';import {refreshTheme} from '$lib/theme';import Group from '$lib/components/Group.svelte';import SelectField from '$lib/components/SelectField.svelte';
let prefs=$state<DesktopPreferences|null>(null);let error=$state('');
onMount(async()=>{try{prefs=await invoke('get_desktop_preferences')}catch(e){error=String(e)}});
async function save(){await invoke('save_desktop_preferences',{preferences:prefs});await refreshTheme();}
useDraft(()=>prefs,save,()=>prefs!==null);
</script>
<Group title="Text" hint="Settings, GTK apps and the bar. Other apps may use their own font settings.">
{#if prefs}<SelectField label="Text size" bind:value={()=>String(prefs!.text_size),v=>prefs!.text_size=Number(v)} options={['12','14','16','18']} labels={{'12':'Small','14':'Default','16':'Large','18':'Larger'}}/><SelectField label="Font" bind:value={prefs.font_family} options={['Varela Round','Inter','Noto Sans']}/>{/if}
{#if error}<p role="alert">{error}</p>{/if}
</Group>
