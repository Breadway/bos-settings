<script lang="ts">
	import TextAppearance from "./TextAppearance.svelte";
	import Hub from "$lib/components/Hub.svelte";
	import Breadpaper from "./Breadpaper.svelte";
	import ShellTheme from "./ShellTheme.svelte";
	import Appearance from "./Windows.svelte";
</script>

<Hub
	title="Appearance"
	lede="Wallpaper sets the colors. Windows follow."
	tabs={[
        { id: "shelltheme", label: "Style", component: ShellTheme },
        { id: "breadpaper", label: "Wallpaper", component: Breadpaper },
        { id: "appearance", label: "Windows", component: Appearance },
        { id: "text", label: "Text", component: TextAppearance },
	]}
/>
