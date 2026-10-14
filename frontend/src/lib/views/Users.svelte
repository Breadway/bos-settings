<script lang="ts">
	import { confirmAction } from "$lib/confirm.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Sheet from "$lib/components/Sheet.svelte";
	import Plus from "@lucide/svelte/icons/plus";

	interface Account {
		username: string;
		full_name: string;
		admin: boolean;
	}

	let accounts = $state<Account[] | null>(null);
	let currentUser = $state("");
	let loadError = $state("");
	let openPasswordFor = $state<string | null>(null);
	let passwordInput = $state("");
	let rowStatus = $state<Record<string, { text: string; bad: boolean }>>({});

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

	function setStatus(username: string, text: string, bad = false) {
		rowStatus = { ...rowStatus, [username]: { text, bad } };
	}

	function subtitle(acc: Account) {
		const parts = [acc.admin ? "Administrator" : "Standard account"];
		if (acc.full_name && acc.full_name !== acc.username) parts.push(acc.username);
		if (acc.username === currentUser) parts.push("you");
		return parts.join(" · ");
	}

	async function refresh() {
		try {
			const info = await invoke<{ accounts: Account[]; current_user: string }>("get_users_info");
			accounts = info.accounts;
			currentUser = info.current_user;
			loadError = "";
		} catch (e) {
			loadError = `Couldn't read the list of people: ${e}`;
			accounts ??= [];
		}
	}

	onMount(refresh);

	async function applyPassword(username: string) {
		if (!passwordInput) return;
		setStatus(username, "Changing password…");
		try {
			await invoke("change_password", { username, password: passwordInput });
			setStatus(username, "Password changed");
			openPasswordFor = null;
			passwordInput = "";
		} catch (e) {
			setStatus(username, `${e}`, true);
			openPasswordFor = null;
		}
	}

	async function removeAccount(username: string) {
		if (!(await confirmAction({ title: `Remove ${username}?`, message: "Deletes the account and its home directory. This cannot be undone.", confirmLabel: "Remove" }))) return;
		try {
			await invoke("remove_user", { username });
		} catch (e) {
			setStatus(username, `Couldn't remove: ${e}`, true);
			return;
		}
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
	{#if accounts === null}
		<p class="muted">Loading…</p>
	{:else}
		{#if loadError}<p class="bad" role="alert">{loadError}</p>{/if}
		<div class="card">
			{#each accounts as acc (acc.username)}
				{@const st = rowStatus[acc.username]}
				<div class="row" data-setting={acc.full_name || acc.username}>
					<span class="av" aria-hidden="true">{(acc.full_name || acc.username).charAt(0).toUpperCase()}</span>
					<div class="lb">
						<b>{acc.full_name || acc.username}</b>
						<small>{subtitle(acc)}</small>
						{#if st}<small class:bad={st.bad} role="status">{st.text}</small>{/if}
					</div>
					<button class="btn" onclick={() => { openPasswordFor = acc.username; passwordInput = ""; confirmInput = ""; }}>Change password</button>
					{#if acc.username !== currentUser}<button class="btn dng" onclick={() => removeAccount(acc.username)}>Remove</button>{/if}
				</div>
			{/each}
			<div class="row"><button class="btn" onclick={() => { adding = true; addStatus = ""; }}><Plus size={15} />Add a person</button></div>
		</div>
	{/if}
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
		<label class="field"><span>Username</span><input class="ui-input" bind:value={newUsername} oninput={() => (usernameTouched = true)} placeholder="alex" /><small>Lowercase letters, numbers, _ and -. Used to log in.</small></label>
		<label class="field"><span>Password</span><input class="ui-input" type="password" autocomplete="new-password" bind:value={newPassword} /></label>
		{#if addStatus}<p class="bad">{addStatus}</p>{/if}
		{#snippet actions()}
			<button class="btn" onclick={() => (adding = false)}>Cancel</button>
			<button class="btn primary" disabled={!newUsername.trim() || !newPassword} onclick={addUser}>Add</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.card { background:var(--surface); border:1px solid var(--line); border-radius:var(--radius); min-width:0; }
	.row { display:flex; align-items:center; gap:12px; padding:11px 16px; min-height:24px; }
	.row + .row { border-top:1px solid var(--line); }
	.row .lb { flex:1; min-width:0; }
	.row .lb b { display:block; font-weight:400; overflow-wrap:anywhere; }
	.row .lb small { display:block; color:var(--muted); font-size:12px; margin-top:1px; }
	.row .lb small.bad { color:var(--red); }
	.av { width:40px; height:40px; border-radius:50%; display:grid; place-items:center; background:linear-gradient(135deg, var(--accent), #6f5fd0); color:#fff; font-size:16px; flex:none; }
	.row .btn { padding:8px 13px; border-radius:9px; font-size:12.5px; line-height:1.45; white-space:nowrap; display:inline-flex; align-items:center; gap:7px; }
	.row .btn.dng { background:none; border-color:color-mix(in oklab, var(--red) 40%, transparent); color:#f0a39c; }
	.row .btn.dng:hover { background:color-mix(in oklab, var(--red) 12%, transparent); }
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px; }
	.field { display:flex; flex-direction:column; gap:6px; padding:8px 0; font-size:13px; }
	.field small { color:var(--muted); font-size:12px; }
	.bad { color:var(--red); font-size:12.5px; margin:4px 0; }
	@media (max-width:560px) { .row { flex-wrap:wrap; } .row .lb { flex-basis:60%; } }
</style>
