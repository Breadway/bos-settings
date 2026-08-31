<script lang="ts">
	import type { Snippet } from "svelte";
	import { setContext } from "svelte";
	import { EMBEDDED_KEY } from "$lib/embed";

	let { children }: { children: Snippet } = $props();
	setContext(EMBEDDED_KEY, true);
</script>

{@render children()}
