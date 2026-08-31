<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Firewall from "./Firewall.svelte";
	import Users from "./Users.svelte";
</script>

<Hub
	title="Privacy & users"
	lede="Firewall and accounts."
	tabs={[
		{ id: "firewall", label: "Firewall", component: Firewall },
		{ id: "users", label: "Users", component: Users },
	]}
/>
