<script lang="ts">
 import {tick} from 'svelte';
 import type {Component} from 'svelte';
 import ViewScaffold from './ViewScaffold.svelte';
 import Embed from './Embed.svelte';
 import {nav} from '$lib/nav.svelte';
 interface Tab{id:string;label:string;component:Component}
 let {title,lede,tabs}:{title:string;lede:string;tabs:Tab[]}=$props();
 let target=$state(nav.tab);
 nav.tab=null;
 let root=$state<HTMLDivElement>();
 $effect(()=>{
  const wanted=nav.tab;
  if(wanted!==null){target=wanted;nav.tab=null;}
 });
 $effect(()=>{
  const id=target;
  if(!id || !root)return;
  void tick().then(()=>{const section=root?.querySelector<HTMLElement>(`[data-tab="${CSS.escape(id)}"]`);section?.scrollIntoView({block:'start'});});
 });
</script>
<ViewScaffold {title} {lede}>
 <div class="sections" bind:this={root}>
 {#each tabs as tab (tab.id)}<section data-tab={tab.id} aria-label={tab.label}><Embed><tab.component/></Embed></section>{/each}
 </div>
</ViewScaffold>
<style>.sections{display:flex;flex-direction:column;gap:14px;min-width:0}section{scroll-margin-top:20px;min-width:0}</style>
