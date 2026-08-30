<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import About from "./About.svelte";
	import DateTime from "./DateTime.svelte";
</script>

<Hub
	title="About"
	lede="This machine and the clock."
	tabs={[
		{ id: "about", label: "Machine", component: About },
		{ id: "datetime", label: "Date & time", component: DateTime },
	]}
/>
