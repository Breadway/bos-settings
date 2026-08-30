<script lang="ts">
	import type { Component } from "svelte";
	import ViewScaffold from "./ViewScaffold.svelte";
	import Subnav from "./Subnav.svelte";
	import Embed from "./Embed.svelte";
	import { nav } from "$lib/nav.svelte";

	interface Tab {
		id: string;
		label: string;
		component: Component;
	}

	let { title, lede, tabs }: { title: string; lede: string; tabs: Tab[] } = $props();

	function takeTab(): string {
		const wanted = nav.tab;
		nav.tab = null;
		if (wanted && tabs.some((t) => t.id === wanted)) return wanted;
		return tabs[0]?.id ?? "";
	}

	let tab = $state(takeTab());

	$effect(() => {
		const wanted = nav.tab;
		if (wanted === null) return;
		if (wanted === "") {
			tab = tabs[0]?.id ?? "";
			nav.tab = null;
		} else if (tabs.some((t) => t.id === wanted)) {
			tab = wanted;
			nav.tab = null;
		}
	});

	let Active = $derived(tabs.find((t) => t.id === tab)?.component);
</script>

<ViewScaffold {title} {lede}>
	{#if tabs.length > 1}
		<Subnav items={tabs.map(({ id, label }) => ({ id, label }))} bind:value={tab} />
	{/if}
	<Embed>
		{#key tab}
			{#if Active}
				<Active />
			{/if}
		{/key}
	</Embed>
</ViewScaffold>
