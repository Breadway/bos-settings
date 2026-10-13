<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Firewall from "./Firewall.svelte";
</script>

<Hub
	title="Privacy & security"
	lede="Who can reach this computer, and what apps can do."
	tabs={[
		{ id: "firewall", label: "Firewall", component: Firewall },
	]}
/>
