<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Network from "./Network.svelte";
	import Vpn from "./Vpn.svelte";
	import NetworkAdvanced from "./NetworkAdvanced.svelte";
</script>

<Hub
	title="Wi-Fi & network"
	lede="Connected networks, places that switch settings for you, and VPN."
	tabs={[
		{ id: "network", label: "Wi-Fi", component: Network },
		{ id: "vpn", label: "Other connections", component: Vpn },
		{ id: "advanced", label: "Advanced", component: NetworkAdvanced },
	]}
/>
