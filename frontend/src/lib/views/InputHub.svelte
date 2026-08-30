<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Keybinds from "./Keybinds.svelte";
	import InputMethod from "./InputMethod.svelte";
	import Accessibility from "./Accessibility.svelte";
</script>

<Hub
	title="Keyboard & mouse"
	lede="Shortcuts, input method, and accessibility."
	tabs={[
		{ id: "keybinds", label: "Shortcuts", component: Keybinds },
		{ id: "ime", label: "Input method", component: InputMethod },
		{ id: "accessibility", label: "Accessibility", component: Accessibility },
	]}
/>
