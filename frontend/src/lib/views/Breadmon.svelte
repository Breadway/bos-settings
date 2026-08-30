<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
</script>

<ViewScaffold title="Monitors">
	<Group
		title="Two different jobs"
		hint="Live arrange, mirror, and named profiles."
	>
		<Hint text="Layout tab is the login config. This opens the live TUI." />
		<button class="primary" onclick={() => invoke("open_breadmon")}>Open breadmon</button>
	</Group>
</ViewScaffold>

<style>
	.primary {
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}
</style>
