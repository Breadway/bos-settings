<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import { crumbs } from "$lib/breadcrumbs.svelte";

	// breadcrumbs pins these DNS servers on every network it joins; empty
	// leaves each network's own DNS alone.
	const DNS: [string, string][] = [["", "Automatic"], ["1.1.1.1", "Cloudflare"], ["9.9.9.9", "Quad9"], ["8.8.8.8", "Google"]];
	let customDns = $state(false);
	let dnsChoice = $derived(customDns || !DNS.some(([v]) => v === (crumbs.cfg?.settings.dns ?? "")) ? "custom" : (crumbs.cfg?.settings.dns ?? ""));

	const WAIT = [5, 8, 10, 15, 20, 30];
	const EVERY = [5, 10, 12, 15, 30, 60];
	function withCurrent(list: number[], v: number) {
		return [...new Set([...list, v])].sort((a, b) => a - b);
	}
</script>

{#if crumbs.cfg}
	{@const s = crumbs.cfg.settings}
	<Group title="Advanced network settings">
		<details class="ui-disclosure">
			<summary>DNS, Tailscale and connection checks</summary>
			<Row label="DNS" hint="Which servers look up website names on every network.">
				<div class="ui-seg">
					{#each DNS as [value, label] (value)}
						<button class:on={dnsChoice === value} onclick={() => { customDns = false; s.dns = value; }}>{label}</button>
					{/each}
					<button class:on={dnsChoice === "custom"} onclick={() => (customDns = true)}>Custom</button>
				</div>
			</Row>
			{#if dnsChoice === "custom"}
				<Row label="DNS servers" hint="Separate several with commas.">
					<input class="ui-input" bind:value={s.dns} placeholder="1.1.1.1, 1.0.0.1" />
				</Row>
			{/if}
			<Row label="Tailscale exit node" hint="Used by places that send traffic through Tailscale, unless the place names its own.">
				<input class="ui-input" bind:value={s.exit_node} placeholder="None" />
			</Row>
			<Row label="Sign-in page check" hint="An address that answers when the internet works, used to spot hotel and school sign-in pages.">
				<input class="ui-input wide" bind:value={s.connectivity_url} />
			</Row>
			<Row label="Reachability check" hint="An address BOS pings to confirm the connection is alive.">
				<input class="ui-input" bind:value={s.ping_host} />
			</Row>
			<Row label="Give up joining a network after">
				<select class="ui-input" bind:value={s.nmcli_wait}>
					{#each withCurrent(WAIT, s.nmcli_wait) as n (n)}<option value={n}>{n} seconds</option>{/each}
				</select>
			</Row>
			<Row label="Check the connection every">
				<select class="ui-input" bind:value={s.watch_interval}>
					{#each withCurrent(EVERY, s.watch_interval) as n (n)}<option value={n}>{n < 60 ? `${n} seconds` : `${n / 60} minute${n === 60 ? "" : "s"}`}</option>{/each}
				</select>
			</Row>
			<ServiceControl unit="breadcrumbs.service" name="Automatic network switching" about="breadcrumbs" bare />
			<Row label="More connection settings" hint="IP addresses, proxies, wired and mobile connections.">
				<button class="btn small" onclick={() => invoke("open_connection_editor")}>Open</button>
			</Row>
		</details>
	</Group>
{/if}

<style>
	.wide { width:260px; }
</style>
