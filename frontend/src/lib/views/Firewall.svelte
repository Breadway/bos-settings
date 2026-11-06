<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import Group from "$lib/components/Group.svelte";
	import { firewallCheckFailure, firewallNotCheckedMessage } from "$lib/firewallCheck";

	interface FirewallRule {
		number: string;
		text: string;
	}
	interface FirewallStatus {
		active: boolean;
		rules: FirewallRule[];
	}

	let details = $state<FirewallStatus | null>(null);
	// Non-root signal only: `systemctl is-active ufw` renders the page
	// without a password prompt (BSET-93).
	let serviceActive = $state<boolean | null>(null);
	let checking = $state(false);
	let checkNotice = $state("");
	let checkError = $state("");
	let newRule = $state("");
	let log = $state<string[]>([]);
	let busy = $state(false);
	// Apps by name instead of raw ufw rules; each needs all its ports open.
	const APPS = [
		{ name: "Remote login (SSH)", hint: "Others can open a terminal on this computer.", specs: ["22/tcp"] },
		{ name: "LocalSend", hint: "Receive files from nearby devices.", specs: ["53317/tcp", "53317/udp"] },
		{ name: "KDE Connect", hint: "Phone link.", specs: ["1714:1764/tcp", "1714:1764/udp"] },
		{ name: "Tailscale", hint: "Faster direct connections on your private network.", specs: ["41641/udp"] },
	];
	const ruleSpecs = (st: FirewallStatus) => new Set(st.rules.filter((r) => /ALLOW/.test(r.text)).map((r) => r.text.trim().split(/\s+/)[0]));
	const allowed = (st: FirewallStatus, specs: string[]) => specs.every((sp) => ruleSpecs(st).has(sp));
	async function setApp(specs: string[], allow: boolean) {
		if (!details) return;
		const have = ruleSpecs(details);
		busy = true; log = [];
		try {
			for (const spec of specs) {
				if (allow && !have.has(spec)) await invoke("add_firewall_rule", { rule: spec });
				if (!allow && have.has(spec)) await invoke("remove_firewall_allow", { rule: spec });
			}
		} catch (e) {
			log = [`${e}`];
		}
		busy = false;
		await refresh();
	}

	onMount(loadSignal);
	async function loadSignal() {
		try {
			serviceActive = await invoke<boolean | null>("get_firewall_service_active");
		} catch {
			serviceActive = null;
		}
	}
	let notChecked = $derived(firewallNotCheckedMessage(serviceActive));

	async function checkFirewall() {
		checking = true;
		checkNotice = "";
		checkError = "";
		try {
			details = await invoke<FirewallStatus>("get_firewall_status");
		} catch (e) {
			const failure = firewallCheckFailure(`${e}`);
			if (failure.declined) checkNotice = failure.message;
			else checkError = failure.message;
		} finally {
			checking = false;
		}
	}

	async function refresh() {
		checkNotice = "";
		checkError = "";
		try {
			details = await invoke<FirewallStatus>("get_firewall_status");
		} catch (e) {
			const failure = firewallCheckFailure(`${e}`);
			if (failure.declined) checkNotice = failure.message;
			else checkError = failure.message;
		}
	}

	async function toggleEnabled(enabled: boolean) {
		if (!details) return;
		log = [];
		try {
			await invoke("set_firewall_enabled", { enabled });
		} catch (e) {
			log = [...log, `${e}`];
		}
		await refresh();
	}

	async function removeRule(number: string) {
		log = [];
		try {
			await invoke("remove_firewall_rule", { number });
		} catch (e) {
			log = [...log, `${e}`];
		}
		await refresh();
	}

	async function addRule() {
		if (!newRule.trim()) return;
		log = [];
		try {
			await invoke("add_firewall_rule", { rule: newRule.trim() });
			newRule = "";
		} catch (e) {
			log = [...log, `${e}`];
		}
		await refresh();
	}
</script>

<ViewScaffold title="Firewall">
	<Group title="Firewall">
		{#if details}
			{@const st = details}
			<SwitchField label="Firewall" hint="Blocks incoming connections you haven't allowed. Changes ask for your password." bind:value={() => st.active, (v) => toggleEnabled(v)} />
			<p class="sub">Allowed to connect in</p>
			{#each APPS as app (app.name)}
				<SwitchField label={app.name} hint={app.hint} disabled={busy} bind:value={() => allowed(st, app.specs), (v) => setApp(app.specs, v)} />
			{/each}
			<details class="ui-disclosure">
				<summary>Advanced rules ({st.rules.length})</summary>
				{#each st.rules as rule (rule.number)}
					<div class="ui-item"><code class="lb">{rule.text}</code><button class="btn small ghost" disabled={busy} onclick={() => removeRule(rule.number)}>Remove</button></div>
				{:else}
					<p class="muted">No extra rules. Incoming is blocked and outgoing is allowed.</p>
				{/each}
				<div class="add-row">
					<input class="ui-input" bind:value={newRule} placeholder="Port, e.g. 8080/tcp" onkeydown={(e) => e.key === "Enter" && addRule()} />
					<button class="btn small" disabled={busy || !newRule.trim()} onclick={addRule}>Allow</button>
				</div>
			</details>
		{:else}
			<div class="ui-item">
				<div class="lb"><b>Firewall</b><small>{notChecked}</small></div>
				<button class="btn small" disabled={checking} onclick={checkFirewall}>{checking ? "Checking…" : "Check firewall"}</button>
			</div>
			<p class="muted">
				Reading the numbered rules asks for the admin password because ufw only reports them to root.
				Nothing is checked until you press the button.
			</p>
		{/if}
		{#if checkNotice}<p class="muted" role="status">{checkNotice}</p>{/if}
		{#if checkError}<p class="muted bad" role="alert">Couldn't check the firewall: {checkError}</p>{/if}
		{#if log.length}<p class="muted bad" role="alert">{log.join(" ")}</p>{/if}
	</Group>
</ViewScaffold>

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; } .bad { color:var(--red); }
	.sub { margin:12px 2px 2px; font-size:11px; letter-spacing:.08em; text-transform:uppercase; color:var(--muted); }
	code { font-size:12px; color:var(--muted); overflow-wrap:anywhere; }
	.add-row { display:flex; gap:8px; padding:6px 0 10px; }
	.add-row input { flex:1; }
</style>
