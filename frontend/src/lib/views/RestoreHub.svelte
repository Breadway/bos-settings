<script lang="ts">
 import Hub from "$lib/components/Hub.svelte";
 import Snapshots from "./Snapshots.svelte";
 import Backup from "./Backup.svelte";
</script>
<Hub title="Backup & restore" lede="System snapshots and backups of your personal files." tabs={[
 { id: "snapshots", label: "System snapshots", component: Snapshots },
 { id: "backup", label: "Your files", component: Backup },
]} />
