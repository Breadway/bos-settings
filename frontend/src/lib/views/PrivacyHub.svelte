<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Firewall from "./Firewall.svelte";
</script>

<Hub
	title="Privacy & security"
	lede="Who can reach this computer."
	tabs={[
		{ id: "firewall", label: "Firewall", component: Firewall },
	]}
/>
