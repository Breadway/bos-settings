<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Breadbar from "./Breadbar.svelte";
	import Breadbox from "./Breadbox.svelte";
	import Breadlock from "./Breadlock.svelte";
	import Breadshot from "./Breadshot.svelte";
	import Autostart from "./Autostart.svelte";
	import DesktopMore from "./DesktopMore.svelte";
</script>

<Hub
	title="Bar & apps"
	lede="Bar, lock screen, screenshots, and startup."
	tabs={[
		{ id: "breadbar", label: "Bar", component: Breadbar },
		{ id: "breadlock", label: "Lock", component: Breadlock },
		{ id: "breadshot", label: "Screenshots", component: Breadshot },
		{ id: "autostart", label: "Startup", component: Autostart },
		{ id: "breadbox", label: "Launcher", component: Breadbox },
		{ id: "more", label: "More", component: DesktopMore },
	]}
/>
