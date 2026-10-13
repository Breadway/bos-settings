<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Plus from "@lucide/svelte/icons/plus";

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
	let confirmInput = $state("");
	let adding = $state(false);
	let usernameTouched = false;
	function suggestUsername() {
		if (usernameTouched) return;
		newUsername = newFullName.trim().split(/\s+/)[0]?.toLowerCase().replace(/[^a-z0-9]/g, "") ?? "";
	}

	async function refresh() {
		const info = await invoke<{ accounts: Account[]; current_user: string }>("get_users_info");
		accounts = info.accounts;
		currentUser = info.current_user;
	}

	onMount(refresh);

	async function applyPassword(username: string) {
		if (!passwordInput) return;
		rowStatus = { ...rowStatus, [username]: "changing password…" };
		try {
			await invoke("change_password", { username, password: passwordInput });
			rowStatus = { ...rowStatus, [username]: "password changed" };
			openPasswordFor = null;
			passwordInput = "";
		} catch {
			rowStatus = { ...rowStatus, [username]: "the password wasn't changed" };
			openPasswordFor = null;
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
			addStatus = "";
			adding = false;
			usernameTouched = false;
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

<ViewScaffold title="Users" lede="People who can log in to this computer.">
	<Group title="People">
		{#if accounts}
			{#each accounts as acc (acc.username)}
				<div class="ui-item">
					<span class="ico av">{(acc.full_name || acc.username).charAt(0).toUpperCase()}</span>
					<div class="lb"><b>{acc.full_name || acc.username}</b><small>{acc.username}{acc.username === currentUser ? " · you" : ""}{rowStatus[acc.username] ? ` · ${rowStatus[acc.username]}` : ""}</small></div>
					<button class="btn small" onclick={() => { openPasswordFor = acc.username; passwordInput = ""; confirmInput = ""; }}>Change password</button>
					{#if acc.username !== currentUser}<button class="btn small remove" onclick={() => removeAccount(acc.username)}>Remove</button>{/if}
				</div>
			{/each}
		{/if}
		<div class="foot"><button class="btn small" onclick={() => { adding = true; addStatus = ""; }}><Plus size={14} />Add a person</button></div>
	</Group>
</ViewScaffold>

{#if openPasswordFor}
	{@const who = openPasswordFor}
	<Sheet title="New password for {who}" onClose={() => (openPasswordFor = null)}>
		<label class="field"><span>New password</span><input class="ui-input" type="password" autocomplete="new-password" bind:value={passwordInput} /></label>
		<label class="field"><span>Type it again</span><input class="ui-input" type="password" autocomplete="new-password" bind:value={confirmInput} /></label>
		{#if confirmInput && confirmInput !== passwordInput}<p class="bad">The two passwords don't match.</p>{/if}
		{#snippet actions()}
			<button class="btn" onclick={() => (openPasswordFor = null)}>Cancel</button>
			<button class="btn primary" disabled={!passwordInput || passwordInput !== confirmInput} onclick={() => applyPassword(who)}>Change password</button>
		{/snippet}
	</Sheet>
{/if}

{#if adding}
	<Sheet title="Add a person" lede="They get their own home folder and settings." onClose={() => (adding = false)}>
		<label class="field"><span>Name</span><input class="ui-input" bind:value={newFullName} oninput={suggestUsername} placeholder="Alex Smith" /></label>
		<label class="field"><span>Username</span><input class="ui-input" bind:value={newUsername} oninput={() => (usernameTouched = true)} placeholder="alex" /><small>Lowercase letters and numbers. Used to log in.</small></label>
		<label class="field"><span>Password</span><input class="ui-input" type="password" autocomplete="new-password" bind:value={newPassword} /></label>
		{#if addStatus}<p class="bad">{addStatus}</p>{/if}
		{#snippet actions()}
			<button class="btn" onclick={() => (adding = false)}>Cancel</button>
			<button class="btn primary" disabled={!newUsername.trim() || !newPassword} onclick={addUser}>Add</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.av { font-weight:600; }
	.foot { padding:6px 0 10px; }
	.foot .btn { display:inline-flex; align-items:center; gap:6px; }
	.field { display:flex; flex-direction:column; gap:6px; padding:8px 0; font-size:13px; }
	.field small { color:var(--muted); font-size:12px; }
	.bad { color:var(--red); font-size:12.5px; margin:4px 0; }
</style>
