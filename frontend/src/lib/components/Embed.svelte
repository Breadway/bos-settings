<script lang="ts">
	import type { Snippet } from "svelte";
	import { setContext } from "svelte";
	import { EMBEDDED_KEY } from "$lib/embed";
	import SectionError from "./SectionError.svelte";

	let { children }: { children: Snippet } = $props();
	setContext(EMBEDDED_KEY, true);
</script>

<!-- A section that throws while rendering must not take its siblings down with it:
     without a boundary one failing section aborts the whole hub page, so the
     other sections never mount and sit on "Loading..." forever. -->
<svelte:boundary onerror={(e) => console.error("section failed", e)}>
	{@render children()}
	{#snippet failed(error, reset)}
		<SectionError {error} {reset} />
	{/snippet}
</svelte:boundary>
