<script lang="ts">
	import {useDraft} from "$lib/drafts.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import { localeLabels } from "$lib/localeNames";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import Group from "$lib/components/Group.svelte";
	import Row from "$lib/components/Row.svelte";

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

	// "en_AU.UTF-8" -> "9/10/2026 · 1,234.56": how dates and numbers will look.
	function sample(region: string): string {
		const tag = region.split(".")[0].replace("_", "-");
		try {
			return `${new Date().toLocaleDateString(tag)} · ${(1234.56).toLocaleString(tag)}`;
		} catch {
			return "Dates, numbers, currency and measurements.";
		}
	}

	async function toggleNtp(enabled: boolean) {
		try {await invoke("set_ntp_enabled", { enabled });ntpEnabled=enabled;}catch(e){tzStatus=String(e);}
	}
</script>

<ViewScaffold title="Date & language" lede="Time, region and language.">
	<Group title="Time">
		{#if info}
			<SwitchField label="Set the time automatically" hint={ntpEnabled ? (info.ntp_synced ? `Synced · now ${info.current_time}` : "Waiting for the network to sync") : `Now ${info.current_time}`} bind:value={() => ntpEnabled, (v) => toggleNtp(v)} />
			{#if info.timezones.length}
				<Row label="Time zone" hint={tzStatus || undefined}>
					<select class="ui-input" value={info.current_tz} onchange={(e) => applyTimezone(e.currentTarget.value)}>
						{#each info.timezones as tz (tz)}<option value={tz}>{tz.replaceAll("_", " ")}</option>{/each}
					</select>
				</Row>
			{:else}
				<p class="muted">Time zones couldn't be listed on this system.</p>
			{/if}
		{/if}
		{#if clockReady}
			<Row label="Clock">
				<div class="ui-seg"><button class:on={clock === "24"} onclick={() => (clock = "24")}>24-hour</button><button class:on={clock === "12"} onclick={() => (clock = "12")}>12-hour</button></div>
			</Row>
		{/if}
	</Group>
	{#if locale}
		<Group title="Language and region" hint="Changes here apply the next time you log in.">
			<SelectField label="Language" bind:value={locale.language} options={Array.from(new Set([...locale.locales, locale.language]))} labels={localeLabels([...locale.locales, locale.language])} />
			<SelectField label="Formats" hint={sample(locale.region)} bind:value={locale.region} options={Array.from(new Set([...locale.locales, locale.region]))} labels={localeLabels([...locale.locales, locale.region])} />
		</Group>
	{/if}
	{#if localeError}<p class="muted bad" role="alert">{localeError}</p>{/if}
</ViewScaffold>

<style>
	.muted { color:var(--muted); font-size:12.5px; margin:4px 2px 8px; } .bad { color:var(--red); }
</style>
