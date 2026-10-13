<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Printer from "@lucide/svelte/icons/printer";

	interface PrinterRow {
		name: string;
		status: string;
		enabled: boolean;
		is_default: boolean;
	}
	interface PrintingStatus {
		printers: PrinterRow[];
		default: string | null;
		cups_ok: boolean;
		error: string | null;
	}

	let status = $state<PrintingStatus | null>(null);
	let message = $state("");
	let address = $state("");
	function printerState(p: PrinterRow): string {
		if (!p.enabled) return "Turned off";
		const st = p.status.toLowerCase();
		return st.includes("idle") ? "Ready" : st.includes("print") ? "Printing" : p.status.charAt(0).toUpperCase() + p.status.slice(1);
	}

	async function refresh() {
		status = await invoke<PrintingStatus>("get_printers");
	}

	onMount(refresh);

	async function setDefault(name: string) {
		message = "";
		try {
			await invoke("set_default_printer", { name });
			message = `${name} is the default printer.`;
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
	}

	// An address or full URI -> an IPP Everywhere queue named after the host.
	async function addByAddress() {
		message = "";
		const raw = address.trim();
		const uri = raw.includes("://") ? raw : `ipp://${raw}/ipp/print`;
		const host = uri.replace(/^\w+:\/\//, "").split(/[/:]/)[0] ?? "printer";
		const name = `Printer_${host}`.replace(/[^A-Za-z0-9_-]/g, "_");
		try {
			await invoke("add_ipp_printer", { name, uri });
			message = `Added the printer at ${host}.`;
			address = "";
			await refresh();
		} catch (e) {
			message = `Couldn't add it: ${e}`;
		}
	}
</script>

<ViewScaffold title="Printers" lede="Printers on your network usually show up on their own.">
	<Group title="Your printers">
		{#if !status}
			<p class="muted">Loading…</p>
		{:else if status.error}
			<p class="muted bad">The printing service isn't answering. {status.error}</p>
		{:else if status.printers.length === 0}
			<p class="muted">No printers yet. Add one below.</p>
		{:else}
			{#each status.printers as p (p.name)}
				<div class="ui-item">
					<span class="ico"><Printer size={18} /></span>
					<div class="lb"><b>{p.name.replace(/[_-]+/g, " ")}</b><small>{printerState(p)}{p.is_default ? " · default printer" : ""}</small></div>
					{#if !p.is_default && status.printers.length > 1}<button class="btn small ghost" onclick={() => setDefault(p.name)}>Make default</button>{/if}
				</div>
			{/each}
		{/if}
		{#if message}<p class="muted" role="status">{message}</p>{/if}
	</Group>
	<Group title="Add a printer">
		<div class="ui-item">
			<div class="lb"><b>Find a printer</b><small>Looks on your network and USB, and installs drivers.</small></div>
			<button class="btn small primary" onclick={() => invoke("open_printer_settings")}>Open</button>
		</div>
		<details class="ui-disclosure">
			<summary>Add by address</summary>
			<div class="addr">
				<input class="ui-input" bind:value={address} placeholder="192.168.1.20 or ipp://printer.local/ipp/print" />
				<button class="btn small" disabled={!address.trim()} onclick={addByAddress}>Add</button>
			</div>
			<p class="muted">For printers that aren't found by themselves. Works with most printers made after 2015.</p>
		</details>
	</Group>
</ViewScaffold>

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; line-height:1.45; } .bad { color:var(--red); }
	.addr { display:flex; gap:8px; padding:4px 0; }
	.addr input { flex:1; }
</style>
