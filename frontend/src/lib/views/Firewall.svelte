<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Row from "$lib/components/Row.svelte";
	import Group from "$lib/components/Group.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import LogView from "$lib/components/LogView.svelte";
	import Shield from "@lucide/svelte/icons/shield";
	import ShieldAlert from "@lucide/svelte/icons/shield-alert";

	interface FirewallRule {
		number: string;
		text: string;
	}
	interface FirewallStatus {
		active: boolean;
		rules: FirewallRule[];
	}

	let status = $state<FirewallStatus | "unloaded" | { error: string }>("unloaded");
	let enabledSensitive = $state(false);
	let newRule = $state("");
	let log = $state<string[]>([]);

	onMount(refresh);

	function friendlyError(raw: string): string {
		const s = raw.toLowerCase();
		if (
			s.includes("polkit") ||
			s.includes("pkexec") ||
			s.includes("password") ||
			s.includes("authentication") ||
			s.includes("controlling terminal")
		) {
			return "Need your password to read firewall rules.";
		}
		return raw;
	}

	async function refresh() {
		try {
			status = await invoke<FirewallStatus>("get_firewall_status");
			enabledSensitive = true;
		} catch (e) {
			status = { error: friendlyError(`${e}`) };
		}
	}

	async function toggleEnabled(enabled: boolean) {
		if (typeof status !== "object" || !("active" in status)) return;
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
	<Group title="Firewall" hint="Needs your password.">
		<Row label="Firewall enabled">
			<button
				class="switch"
				class:on={typeof status === "object" && "active" in status && status.active}
				disabled={!enabledSensitive}
				role="switch"
				aria-checked={typeof status === "object" && "active" in status && status.active}
				aria-label="Firewall enabled"
				onclick={() => toggleEnabled(!(typeof status === "object" && "active" in status && status.active))}
			>
				<span class="knob"></span>
			</button>
		</Row>
	</Group>

	<Group title="Add rule" hint={'e.g. "8080/tcp", "22/tcp", or a service name like "OpenSSH".'}>
		<div class="add-row">
			<input type="text" bind:value={newRule} placeholder="port/proto or service name" />
			<button class="allow" onclick={addRule}>Allow</button>
		</div>
	</Group>

	<Group title="Rules" wide>
		<div class="list">
			{#if status === "unloaded"}
				<EmptyState icon={Shield} title="Loading…" hint="May ask for your password." />
			{:else if "error" in status}
				<EmptyState icon={ShieldAlert} title="Couldn't read firewall status" hint={status.error} />
			{:else if status.rules.length === 0}
				<EmptyState icon={Shield} title="No rules" hint="Only the default policy (deny incoming, allow outgoing) applies." />
			{:else}
				{#each status.rules as rule (rule.number)}
					<div class="rule-row">
						<span class="text">{rule.text}</span>
						<button class="remove" onclick={() => removeRule(rule.number)}>Remove</button>
					</div>
				{/each}
			{/if}
		</div>

		<button class="refresh" onclick={refresh}>Refresh status</button>
	</Group>

	<LogView lines={log} />
</ViewScaffold>

<style>
	.switch {
		width: 40px;
		height: 22px;
		border-radius: 999px;
		border: none;
		background-color: var(--overlay);
		padding: 2px;
		cursor: pointer;
		display: flex;
		align-items: center;
	}
	.switch.on {
		background-color: var(--accent);
		justify-content: flex-end;
	}
	.switch:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
	}

	.list {
		min-height: 60px;
		max-height: 260px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.rule-row {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		background-color: var(--surface);
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
	}

	.text {
		flex: 1;
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
	}

	.refresh {
		background-color: var(--surface);
		color: var(--on-surface);
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
	}

	.add-row {
		display: flex;
		gap: var(--space-sm, 8px);
	}

	.add-row input {
		flex: 1;
		background-color: var(--surface);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.allow {
		background-color: var(--accent);
		color: var(--on-accent);
	}
</style>
