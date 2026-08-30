<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Display from "./Display.svelte";
	import NightLight from "./NightLight.svelte";
	import Breadmon from "./Breadmon.svelte";
</script>

<Hub
	title="Displays"
	lede="Drag to arrange. Scale and rotate apply now."
	tabs={[
		{ id: "hyprland", label: "Layout", component: Display },
		{ id: "nightlight", label: "Night light", component: NightLight },
		{ id: "breadmon", label: "Live arrange", component: Breadmon },
	]}
/>
