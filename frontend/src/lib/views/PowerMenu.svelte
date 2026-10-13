<script lang="ts">
import {invoke} from '@tauri-apps/api/core';import {go} from '$lib/nav.svelte';import ViewScaffold from '$lib/components/ViewScaffold.svelte';import Group from '$lib/components/Group.svelte';
let busy=$state(false);let error=$state('');
async function run(action:'sleep'|'restart'|'power_off',label:string){if(busy || !confirm(`${label} this computer?`))return;busy=true;try{await invoke('run_power_action',{action})}catch(e){error=String(e)}finally{busy=false}}
</script>
<ViewScaffold title="Power" lede="Choose what happens next."><Group title="Power"><button class="btn" disabled={busy} onclick={()=>run('sleep','Put to sleep')}>Sleep</button><button class="btn" disabled={busy} onclick={()=>run('restart','Restart')}>Restart</button><button class="btn danger" disabled={busy} onclick={()=>run('power_off','Power off')}>Power off</button><button class="btn" onclick={()=>go('home')}>Cancel</button>{#if error}<p role="alert">{error}</p>{/if}</Group></ViewScaffold>
