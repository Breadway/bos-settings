<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import TextField from "$lib/components/TextField.svelte";
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
	let newName = $state("");
	let newUri = $state("");

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

	async function addIpp() {
		message = "";
		try {
			await invoke("add_ipp_printer", { name: newName.trim(), uri: newUri.trim() });
			message = `Added ${newName.trim()}.`;
			newName = "";
			newUri = "";
			await refresh();
		} catch (e) {
			message = `${e}`;
		}
	}
</script>

<ViewScaffold title="Printing">
	<Group title="Printers" hint="CUPS + Avahi ship on the ISO. Discovery and drivers for odd hardware live in system-config-printer." wide>
		{#if !status}
			<Hint text="Loading…" />
		{:else if status.error}
			<EmptyState icon={Printer} title="Couldn't talk to CUPS" hint={status.error} />
		{:else if status.printers.length === 0}
			<EmptyState icon={Printer} title="No printers yet" hint="Add one below, or open the CUPS printer wizard." />
		{:else}
			<div class="list">
				{#each status.printers as p (p.name)}
					<div class="row">
						<div class="meta">
							<span class="name">{p.name}{p.is_default ? " (default)" : ""}</span>
							<span class="status">{p.enabled ? p.status : "disabled"}</span>
						</div>
						<button disabled={p.is_default} onclick={() => setDefault(p.name)}>Set default</button>
					</div>
				{/each}
			</div>
		{/if}
		<div class="btn-row">
			<button onclick={refresh}>Refresh</button>
			<button class="primary" onclick={() => invoke("open_printer_settings")}>Add printer…</button>
		</div>
		{#if message}<Hint text={message} />{/if}
	</Group>

	<Group title="IPP Everywhere" hint="For a printer that already speaks IPP. Name must be letters, digits, dash, underscore.">
		<TextField label="Name" bind:value={newName} placeholder="Office" />
		<TextField label="URI" bind:value={newUri} placeholder="ipp://192.168.1.20/ipp/print" />
		<button disabled={!newName.trim() || !newUri.trim()} onclick={addIpp}>Add IPP printer</button>
	</Group>
</ViewScaffold>

<style>
	.list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-md, 12px);
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.meta {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.status {
		opacity: 0.7;
		font-size: var(--font-size-secondary, 12px);
	}

	.btn-row {
		display: flex;
		gap: var(--space-sm, 8px);
		margin-top: var(--space-sm, 8px);
	}

	button {
		background-color: var(--surface);
		color: var(--on-surface);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	button.primary {
		background-color: var(--accent);
		color: var(--on-accent);
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
