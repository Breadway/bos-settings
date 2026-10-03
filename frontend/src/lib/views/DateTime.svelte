<script lang="ts">
	import {useDraft} from "$lib/drafts.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
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

	let clock=$state("24");let initialClock="24";let clockReady=$state(false);let initialLocale="";
	onMount(async()=>{clock=await invoke<boolean>("get_clock_format")?"24":"12";initialClock=clock;clockReady=true;});
	let locale=$state<{locales:string[];language:string;region:string}|null>(null);
	let localeError=$state("");
	onMount(async()=>{try{locale=await invoke("get_locale_info");initialLocale=JSON.stringify({language:locale!.language,region:locale!.region})}catch(e){localeError=String(e)}});
	async function saveLocale(){if(clock!==initialClock){await invoke("set_clock_format",{clock24h:clock==="24"});initialClock=clock;}const choice={language:locale!.language,region:locale!.region};if(JSON.stringify(choice)!==initialLocale){await invoke("save_locale_choice",{choice});initialLocale=JSON.stringify(choice);}}
	useDraft(()=>({locale,clock}),saveLocale,()=>locale!==null && clockReady);
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
		try {await invoke("set_ntp_enabled", { enabled });ntpEnabled=enabled;}catch(e){tzStatus=String(e);}
	}
</script>

<ViewScaffold title="Date & language" lede="Time, region and language.">
 {#if locale}<Group title="Language and region" hint="Applies at your next login. The list contains the locales already installed on this computer.">
 <SelectField label="Clock format" bind:value={clock} options={["12","24"]} labels={{"12":"12-hour","24":"24-hour"}}/>
 <SelectField label="Language" bind:value={locale.language} options={Array.from(new Set([...locale.locales,locale.language]))}/>
 <SelectField label="Region" hint="Dates, numbers, currency and measurements." bind:value={locale.region} options={Array.from(new Set([...locale.locales,locale.region]))}/>
 </Group><SaveButton onSave={saveLocale}/>{/if}{#if localeError}<p role="alert">{localeError}</p>{/if}
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
