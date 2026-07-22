<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";
	import Hint from "$lib/components/Hint.svelte";

	interface DateTimeInfo {
		current_time: string;
		timezones: string[];
		current_tz: string;
		ntp_enabled: boolean;
		ntp_synced: boolean;
	}

	let info = $state<DateTimeInfo | null>(null);
	let tzStatus = $state("");
	let ntpEnabled = $state(false);

	onMount(async () => {
		info = await invoke<DateTimeInfo>("get_datetime_info");
		ntpEnabled = info.ntp_enabled;
	});

	async function applyTimezone(tz: string) {
		tzStatus = "Applying…";
		try {
			await invoke("set_timezone", { tz });
			tzStatus = "Applied";
		} catch (e) {
			tzStatus = `${e}`;
		}
	}

	async function toggleNtp(enabled: boolean) {
		ntpEnabled = enabled;
		await invoke("set_ntp_enabled", { enabled });
	}
</script>

<ViewScaffold title="Date & Time">
	{#if info}
		<Group title="Current time">
			<InfoRow label="Right now" value={info.current_time} />
		</Group>

		<Group title="Timezone">
			{#if info.timezones.length === 0}
				<Hint text="Couldn't list timezones (is timedatectl available?)." />
			{:else}
				<Row label="Timezone">
					<select value={info.current_tz} onchange={(e) => applyTimezone(e.currentTarget.value)}>
						{#each info.timezones as tz (tz)}
							<option value={tz}>{tz}</option>
						{/each}
					</select>
				</Row>
				{#if tzStatus}
					<span class="status">{tzStatus}</span>
				{/if}
			{/if}
		</Group>

		<Group title="Network time" hint={info.ntp_synced ? "Synchronized." : "Not synchronized yet (needs network)."}>
			<Row label="Synchronize automatically">
				<button
					class="switch"
					class:on={ntpEnabled}
					role="switch"
					aria-checked={ntpEnabled}
					aria-label="Synchronize automatically"
					onclick={() => toggleNtp(!ntpEnabled)}
				>
					<span class="knob"></span>
				</button>
			</Row>
		</Group>
	{/if}
</ViewScaffold>

<style>
	select {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

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

	.knob {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background-color: var(--on-surface);
		display: block;
	}
</style>
