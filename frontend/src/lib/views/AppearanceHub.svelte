<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Breadpaper from "./Breadpaper.svelte";
	import Appearance from "./Appearance.svelte";
</script>

<Hub
	title="Appearance"
	lede="Wallpaper sets the colors. Windows follow."
	tabs={[
		{ id: "breadpaper", label: "Wallpaper", component: Breadpaper },
		{ id: "appearance", label: "Windows", component: Appearance },
	]}
/>
