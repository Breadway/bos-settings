<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Updates from "./Updates.svelte";
	import Packages from "./Packages.svelte";
	import Aur from "./Aur.svelte";
	import Firmware from "./Firmware.svelte";
	import Snapshots from "./Snapshots.svelte";
	import Backup from "./Backup.svelte";
	import Channel from "./Channel.svelte";
</script>

<Hub
	title="Updates & backup"
	lede="Updates, snapshots, and backups."
	tabs={[
		{ id: "updates", label: "Updates", component: Updates },
		{ id: "packages", label: "Packages", component: Packages },
		{ id: "aur", label: "AUR", component: Aur },
		{ id: "firmware", label: "Firmware", component: Firmware },
		{ id: "snapshots", label: "Snapshots", component: Snapshots },
		{ id: "backup", label: "Backup", component: Backup },
		{ id: "channel", label: "Channel", component: Channel },
	]}
/>
