<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";

	interface Account {
		username: string;
		full_name: string;
	}

	let accounts = $state<Account[] | null>(null);
	let currentUser = $state("");
	let openPasswordFor = $state<string | null>(null);
	let passwordInput = $state("");
	let rowStatus = $state<Record<string, string>>({});

	let newUsername = $state("");
	let newFullName = $state("");
	let newPassword = $state("");
	let addStatus = $state("");

	async function refresh() {
		const info = await invoke<{ accounts: Account[]; current_user: string }>("get_users_info");
		accounts = info.accounts;
		currentUser = info.current_user;
	}

	onMount(refresh);

	async function applyPassword(username: string) {
		if (!passwordInput) return;
		rowStatus = { ...rowStatus, [username]: "Applying…" };
		try {
			await invoke("change_password", { username, password: passwordInput });
			rowStatus = { ...rowStatus, [username]: "Password changed" };
			openPasswordFor = null;
			passwordInput = "";
		} catch {
			rowStatus = { ...rowStatus, [username]: "Failed to change password" };
		}
	}

	async function removeAccount(username: string) {
		if (!confirm(`Remove user ${username}? Deletes the account and its home directory. This cannot be undone.`)) return;
		await invoke("remove_user", { username });
		await refresh();
	}

	async function addUser() {
		if (!newUsername.trim() || !newPassword) {
			addStatus = "Username and password are required.";
			return;
		}
		addStatus = "Adding…";
		try {
			await invoke("add_user", { username: newUsername.trim(), fullName: newFullName.trim(), password: newPassword });
			addStatus = "User added.";
			newUsername = "";
			newFullName = "";
			newPassword = "";
			await refresh();
		} catch (e) {
			addStatus = `${e}`;
			await refresh();
		}
	}
</script>

<ViewScaffold title="Users">
	<Group title="Accounts" hint="Login accounts. You cannot delete yourself." wide>
		<div class="list">
			{#if accounts}
				{#each accounts as acc (acc.username)}
					<div class="card">
						<div class="top">
							<span class="name">{acc.username}{acc.full_name ? ` (${acc.full_name})` : ""}</span>
							<button class="action" onclick={() => (openPasswordFor = openPasswordFor === acc.username ? null : acc.username)}>
								Change password
							</button>
							<button class="remove" disabled={acc.username === currentUser} onclick={() => removeAccount(acc.username)}>Remove</button>
						</div>
						{#if openPasswordFor === acc.username}
							<div class="pw-row">
								<input type="password" bind:value={passwordInput} placeholder="New password" />
								<button class="action" onclick={() => applyPassword(acc.username)}>Apply</button>
							</div>
						{/if}
						{#if rowStatus[acc.username]}
							<span class="status">{rowStatus[acc.username]}</span>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	</Group>

	<Group title="Add user">
		<Row label="Username">
			<input type="text" bind:value={newUsername} placeholder="username" />
		</Row>
		<Row label="Full name">
			<input type="text" bind:value={newFullName} placeholder="Full name (optional)" />
		</Row>
		<Row label="Password">
			<input type="password" bind:value={newPassword} placeholder="password" />
		</Row>
		<button class="add" onclick={addUser}>Add user</button>
		{#if addStatus}
			<span class="status">{addStatus}</span>
		{/if}
	</Group>
</ViewScaffold>

<style>
	.list {
		max-height: 320px;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-xs, 4px);
	}

	.card {
		background-color: var(--surface);
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-md, 12px);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.top {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
	}

	.name {
		flex: 1;
	}

	.pw-row {
		display: flex;
		gap: var(--space-sm, 8px);
	}

	input[type="text"],
	input[type="password"] {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.pw-row input {
		flex: 1;
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.action {
		background-color: var(--bg);
		color: var(--on-surface);
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
	}

	.remove:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.add {
		background-color: var(--accent);
		color: var(--on-accent);
		align-self: flex-start;
		margin-top: var(--space-sm, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
