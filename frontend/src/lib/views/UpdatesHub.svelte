<script lang="ts">
 import Hub from "$lib/components/Hub.svelte";
 import Updates from "./Updates.svelte";
 import Firmware from "./Firmware.svelte";
 import Channel from "./Channel.svelte";
</script>
<Hub title="Updates" lede="Keep your system, bread apps and firmware up to date." tabs={[
 { id: "updates", label: "Updates", component: Updates },
 { id: "firmware", label: "Firmware", component: Firmware },
 { id: "channel", label: "Channel", component: Channel },
]} />
