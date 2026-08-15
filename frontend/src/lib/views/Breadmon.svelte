<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
</script>

<ViewScaffold title="Monitors">
	<Group
		title="Two different jobs"
		hint="This is not a second Display editor. Display (monitors.json) is the login-time layout Hyprland itself reads. breadmon is a TUI for live arrange / mirror / named profiles."
	>
		<Hint
			text="Use Display when you want the persistent Hyprland rule set (output / mode / position / scale) that applies on next login or hyprctl reload."
		/>
		<Hint
			text="Use breadmon when you want to drag monitors around live, pick a common mirror mode, or save/load a profile under ~/.config/breadmon/profiles/. Settings never writes those profiles."
		/>
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
