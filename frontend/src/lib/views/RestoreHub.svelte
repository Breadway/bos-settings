<script lang="ts">
 import ViewScaffold from '$lib/components/ViewScaffold.svelte';
 import Embed from '$lib/components/Embed.svelte';
 import Snapshots from './Snapshots.svelte';
 import Backup from './Backup.svelte';
</script>
<ViewScaffold title="Backup & restore" lede="Go back to an earlier system, or bring back your files.">
 <Embed><Snapshots/><Backup/></Embed>
</ViewScaffold>
