<script lang="ts">
 import Hub from "$lib/components/Hub.svelte";
 import Breadbar from "./Breadbar.svelte";
 import Breadbox from "./Breadbox.svelte";
 import Breadshot from "./Breadshot.svelte";
 import Breadclip from "./Breadclip.svelte";
 import Breadpad from "./Breadpad.svelte";
 import Breadsearch from "./Breadsearch.svelte";
 import Bread from "./Bread.svelte";
 import Breadhelp from "./Breadhelp.svelte";
</script>
<Hub title="Desktop & bar" lede="Bar, launcher, screenshots and desktop tools." tabs={[
 { id: "breadbar", label: "Bar", component: Breadbar },
 { id: "breadbox", label: "Launcher", component: Breadbox },
 { id: "breadshot", label: "Screenshots", component: Breadshot },
 { id: "breadclip", label: "Clipboard", component: Breadclip },
 { id: "breadpad", label: "Notes", component: Breadpad },
 { id: "breadsearch", label: "File search", component: Breadsearch },
 { id: "bread", label: "Automations", component: Bread },
 { id: "breadhelp", label: "Help", component: Breadhelp },
]} />
