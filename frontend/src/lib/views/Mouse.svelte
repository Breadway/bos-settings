<script lang="ts">
 import {onMount} from 'svelte';
 import {invoke} from '@tauri-apps/api/core';
 import type {DesktopPreferences} from '$lib/desktopPreferences';
 import {useDraft} from '$lib/drafts.svelte';
 import ViewScaffold from '$lib/components/ViewScaffold.svelte';
 import Group from '$lib/components/Group.svelte';
 import SliderField from '$lib/components/SliderField.svelte';
 import SwitchField from '$lib/components/SwitchField.svelte';
 import SelectField from '$lib/components/SelectField.svelte';
 import SaveButton from '$lib/components/SaveButton.svelte';
 import Appearance from './Appearance.svelte';
 let prefs=$state<DesktopPreferences|null>(null);
 let error=$state('');
 onMount(async()=>{try{prefs=await invoke('get_desktop_preferences')}catch(e){error=String(e)}});
 async function save(){await invoke('save_desktop_preferences',{preferences:prefs})}
 useDraft(()=>prefs,save,()=>prefs!==null);
</script>
<ViewScaffold title="Mouse & touchpad" lede="Pointer speed, scrolling and touchpad gestures.">
 {#if prefs}
 <Group title="Pointer">
  <SliderField label="Speed" bind:value={prefs.pointer_speed} min={-1} max={1} step={0.05}/>
 </Group>
 <Group title="Touchpad">
  <SwitchField label="Tap to click" bind:value={prefs.tap_to_click}/>
  <SwitchField label="Natural scrolling" bind:value={prefs.natural_scroll}/>
  <SwitchField label="Disable while typing" bind:value={prefs.disable_while_typing}/>
  <SelectField label="Click method" bind:value={prefs.click_method} options={['clickfinger','buttonareas']} labels={{clickfinger:'Click with fingers',buttonareas:'Bottom corners'}}/>
  <SliderField label="Scroll speed" bind:value={prefs.scroll_factor} min={0.1} max={5} step={0.1}/>
 </Group>
 <details><summary>Focus and existing input settings</summary><Appearance scope="mouse"/></details>
 <SaveButton onSave={save}/>
 {/if}
 {#if error}<p role="alert">{error}</p>{/if}
</ViewScaffold>
