<script lang="ts">
	import Subnav from "$lib/components/Subnav.svelte";
	import Embed from "$lib/components/Embed.svelte";
	import Breadclip from "./Breadclip.svelte";
	import Breadpad from "./Breadpad.svelte";
	import Breadsearch from "./Breadsearch.svelte";
	import Bread from "./Bread.svelte";
	import Breadhelp from "./Breadhelp.svelte";

	const tabs = [
		{ id: "breadclip", label: "Clipboard", component: Breadclip },
		{ id: "breadpad", label: "Notes", component: Breadpad },
		{ id: "breadsearch", label: "Search", component: Breadsearch },
		{ id: "bread", label: "Daemon", component: Bread },
		{ id: "breadhelp", label: "Help", component: Breadhelp },
	];

	let tab = $state(tabs[0].id);
	let Active = $derived(tabs.find((t) => t.id === tab)?.component);
</script>

<Subnav items={tabs.map(({ id, label }) => ({ id, label }))} bind:value={tab} />
<Embed>
	{#key tab}
		{#if Active}
			<Active />
		{/if}
	{/key}
</Embed>
