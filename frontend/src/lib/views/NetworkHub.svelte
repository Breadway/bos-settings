<script lang="ts">
	import Hub from "$lib/components/Hub.svelte";
	import Network from "./Network.svelte";
	import Vpn from "./Vpn.svelte";
	import Breadcrumbs from "./Breadcrumbs.svelte";
</script>

<Hub
	title="Wi-Fi & internet"
	lede="Wi-Fi, saved networks, and VPN."
	tabs={[
		{ id: "network", label: "Wi-Fi", component: Network },
		{ id: "vpn", label: "VPN", component: Vpn },
		{ id: "breadcrumbs", label: "Saved networks", component: Breadcrumbs },
	]}
/>
