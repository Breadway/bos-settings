<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Display from "./Display.svelte";
	import NightLight from "./NightLight.svelte";
</script>

<Hub
	title="Displays"
	lede="Drag to arrange. Every change can be kept or reverted."
	tabs={[
		{ id: "hyprland", label: "Layout", component: Display },
		{ id: "nightlight", label: "Night light", component: NightLight },
	]}
/>
