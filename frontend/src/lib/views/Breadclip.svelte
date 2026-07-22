<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";

	function openHistory() {
		invoke("open_breadclip");
	}
</script>

<ViewScaffold title="Clipboard">
	<Group
		title="Clipboard history"
		hint="Keeps a history of copied text/images and shows it as a popup. breadclipd (right) is the background daemon that watches the clipboard — breadclip itself is just the popup UI, launched on demand by the keybind or the button below."
	>
		<button class="open-btn" onclick={openHistory}>Open history (SUPER+V)</button>
	</Group>
	<ServiceControl unit="breadclipd.service" />
</ViewScaffold>

<style>
	.open-btn {
		align-self: flex-start;
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}
</style>
