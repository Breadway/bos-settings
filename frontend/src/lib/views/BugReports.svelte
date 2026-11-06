<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { useDraft } from "$lib/drafts.svelte";
	import { confirmAction } from "$lib/confirm.svelte";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import EmptyState from "$lib/components/EmptyState.svelte";
	import Bug from "@lucide/svelte/icons/bug";

	interface AppChoice {
		id: string;
		installed: boolean;
		excluded: boolean;
	}
	interface Settings {
		breadbug_installed: boolean;
		upload_enabled: boolean;
		notify_fixed: boolean;
		endpoint: string;
		apps: AppChoice[];
		store_bytes: number;
		queued: number;
		consent_text: string;
	}
	interface HistoryRow {
		t: number;
		kind: "sent" | "saved";
		app: string;
		receipt: string | null;
		status: string | null;
		fixed_in: string | null;
		path: string | null;
	}

	let s = $state<Settings | null>(null);
	/** Per app: included in reports (the inverse of "excluded"), so a switch can bind it. */
	let included = $state<Record<string, boolean>>({});
	let history = $state<HistoryRow[]>([]);
	let error = $state("");
	let notice = $state("");
	let busy = $state(false);

	async function load() {
		try {
			s = await invoke<Settings>("get_breadbug_settings");
			included = Object.fromEntries(s.apps.map((a) => [a.id, !a.excluded]));
			history = await invoke<HistoryRow[]>("breadbug_history");
		} catch (e) {
			error = String(e);
		}
	}
	onMount(load);

	async function save() {
		if (!s) return;
		// Turning sending off while reports wait in the offline queue: ask what
		// to do with them instead of leaving them to be sent later by surprise.
		if (!s.upload_enabled && s.queued > 0) {
			const discard = await confirmAction({
				title: "Discard the waiting reports?",
				message: `${s.queued} report(s) are waiting to be sent. Sending is now off. Discarding deletes them; keeping them holds them until you turn sending back on.`,
				confirmLabel: "Discard them",
				cancelLabel: "Keep them",
			});
			if (discard) {
				await invoke("breadbug_clear_queue");
				s.queued = 0;
			}
		}
		await invoke("save_breadbug_settings", {
			uploadEnabled: s.upload_enabled,
			notifyFixed: s.notify_fixed,
			excludedApps: Object.entries(included)
				.filter(([, inc]) => !inc)
				.map(([id]) => id),
		});
	}
	useDraft(
		() => s && { u: s.upload_enabled, n: s.notify_fixed, x: { ...included } },
		save,
		() => s !== null,
	);

	function when(t: number): string {
		return new Date(t * 1000).toLocaleString();
	}
	function size(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KiB`;
		return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
	}

	async function deleteReport(receipt: string) {
		const ok = await confirmAction({
			title: "Delete this report from the server?",
			message: "This cannot be undone.",
			confirmLabel: "Delete from server",
		});
		if (!ok) return;
		busy = true;
		notice = "";
		try {
			notice = await invoke<string>("breadbug_delete_report", { receipt });
			await load();
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}

	async function wipe() {
		const ok = await confirmAction({
			title: "Delete local data?",
			message: "Delete all the logs and crash information breadbug has stored on this computer?",
			confirmLabel: "Delete",
		});
		if (!ok) return;
		try {
			await invoke("breadbug_wipe_local");
			await load();
			notice = "Local data deleted.";
		} catch (e) {
			error = String(e);
		}
	}

	async function discardQueue() {
		const ok = await confirmAction({
			title: "Discard the reports waiting to be sent?",
			confirmLabel: "Discard",
		});
		if (!ok) return;
		await invoke("breadbug_clear_queue");
		await load();
	}

	async function openReporter() {
		try {
			await invoke("open_breadbug");
		} catch (e) {
			error = String(e);
		}
	}

	const intro = $derived(s ? s.consent_text.split("\n\n") : []);
</script>

<ViewScaffold
	title="Bug reports"
	lede="Report a crash or a problem. You see exactly what would be included, and nothing is sent unless you choose to."
>
	{#if s}
		{#if !s.breadbug_installed}
			<EmptyState
				icon={Bug}
				title="breadbug is not installed"
				hint="Update bread from Updates to get the bug reporter."
			/>
		{/if}

		<Group title="Bug reports">
			<SwitchField
				label="Allow sending bug reports"
				hint="Off by default. Even when on, each report is only sent when you press Send."
				bind:value={s.upload_enabled}
			/>
			<SwitchField
				label="Tell me when a bug I reported is fixed"
				hint="Checks only the reports you sent, once a day, and only while sending is on."
				bind:value={s.notify_fixed}
			/>
			<Row label="Report a problem now" hint="The same as pressing Super + Shift + B.">
				<button class="btn" onclick={openReporter} disabled={!s.breadbug_installed}>Open the reporter</button>
			</Row>
		</Group>

		<Group title="Apps included in reports">
			{#each s.apps as app (app.id)}
				<SwitchField
					label={app.id}
					hint={app.installed ? undefined : "Not installed"}
					bind:value={included[app.id]}
				/>
			{/each}
			{#if !s.apps.length}<p>Apps appear here once they are installed.</p>{/if}
			<p class="muted">An app you turn off here has nothing stored about it, and its existing data is deleted.</p>
		</Group>

		<Group title="What a report contains">
			{#each intro as para}<p>{para}</p>{/each}
		</Group>

		<Group title="Your reports">
			{#each history as h (h.t + (h.receipt ?? h.path ?? ""))}
				<Row
					label={`${h.app}: ${h.kind === "sent" ? "sent" : "saved to a file"}`}
					hint={[
						when(h.t),
						h.kind === "sent" && h.status ? h.status + (h.fixed_in ? ` in ${h.fixed_in}` : "") : "",
						h.path ?? "",
					]
						.filter(Boolean)
						.join(" · ")}
				>
					{#if h.kind === "sent" && h.receipt}
						<code>{h.receipt.slice(0, 8)}</code>
						<button class="btn small ghost" disabled={busy} onclick={() => deleteReport(h.receipt!)}>Delete from server</button>
					{/if}
				</Row>
			{/each}
			{#if !history.length}<p>No reports sent or saved yet.</p>{/if}
		</Group>

		<Group title="On this computer">
			<Row label="Stored logs and crash information" hint={`${size(s.store_bytes)}. Kept only on this computer until you send a report.`}>
				<button class="btn small ghost" onclick={wipe}>Delete local data</button>
			</Row>
			<Row label="Waiting to be sent" hint={s.queued ? `${s.queued} report(s) will be retried while sending is on.` : "Nothing is waiting."}>
				{#if s.queued}<button class="btn small ghost" onclick={discardQueue}>Discard</button>{/if}
			</Row>
		</Group>
	{/if}

	{#if notice}<p role="status">{notice}</p>{/if}
	{#if error}<p role="alert">{error}</p>{/if}
</ViewScaffold>

<style>
	.muted {
		color: var(--muted);
		font-size: 13px;
	}
	code {
		font-size: 12px;
		color: var(--muted);
	}
</style>
