<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Defaults from "./Defaults.svelte";
	import Optional from "./Optional.svelte";
	import Printing from "./Printing.svelte";
</script>

<Hub
	title="Default apps"
	lede="Browser, files, terminal, and extras."
	tabs={[
		{ id: "defaults", label: "Defaults", component: Defaults },
		{ id: "optional", label: "Optional", component: Optional },
		{ id: "printing", label: "Printing", component: Printing },
	]}
/>
