<script lang="ts">
 import Hub from "$lib/components/Hub.svelte";
 import Optional from "./Optional.svelte";
 import Aur from "./Aur.svelte";
 import Packages from "./Packages.svelte";
</script>
<Hub title="Get apps" lede="Find software and manage installed packages." tabs={[
 { id: "optional", label: "Recommended", component: Optional },
 { id: "aur", label: "AUR", component: Aur },
 { id: "packages", label: "Installed", component: Packages },
]} />
