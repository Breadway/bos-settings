<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
</script>

<ViewScaffold title="Clipboard">
	<Group title="Clipboard">
		<div class="ui-item">
			<div class="lb"><b>Clipboard history</b><small>Super + V shows what you copied recently.</small></div>
			<button class="btn small" onclick={() => invoke("open_breadclip")}>Open</button>
		</div>
		<ServiceControl unit="breadclipd.service" name="Keep clipboard history" bare />
	</Group>
</ViewScaffold>
