<script lang="ts">
	import {useDraft} from "$lib/drafts.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import LogView from "$lib/components/LogView.svelte";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Row from "$lib/components/Row.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import { createLiveGate, pollWhileVisible } from "$lib/liveGate";

	interface BatteryProtection {
		kind: "thresholds" | "charge_types" | "conservation";
		enabled: boolean;
		limit_pct: number | null;
	}

	interface PowerInfo {
		protection: BatteryProtection | null;
		battery: [string, string][];
		power_source: string;
		brightness_pct: number | null;
		charge_start: number | null;
		charge_end: number | null;
		tlp_profile: string | null;
	}

	let policy=$state<{dim:number;screen_off:number;lock:number;sleep:number;sleep_on_battery:boolean;lid:string;power_button:string}|null>(null);
 let modes=$state<{backend:string;current:string;profiles:string[];tlp_setup_available:boolean}|null>(null);
 let setupLog=$state<string[]>([]);
 // TLP users: install tlp-pd (allow-listed) and start its service so modes work.
 async function setupModes(){if(modeBusy)return;modeBusy=true;modeError='';setupLog=[];try{const ok=await runStreamed('pacman_install',{packages:['tlp-pd']},l=>{setupLog=[...setupLog,l]});if(ok===false)throw new Error('Installing tlp-pd failed. See the log.');await invoke('enable_tlp_power_profiles');modes=await invoke('get_power_modes')}catch(e){modeError=String(e)}finally{modeBusy=false}}
 let automatic=$state(false);
 onMount(async()=>{automatic=await invoke("get_automatic_power_mode")});
 async function setAutomatic(enabled:boolean){if(modeBusy)return;modeBusy=true;modeError="";try{await invoke("set_automatic_power_mode",{enabled});automatic=enabled;}catch(e){modeError=String(e)}finally{modeBusy=false}}
 let modeBusy=$state(false);let modeError=$state('');
 onMount(async()=>{policy=await invoke('get_power_settings');modes=await invoke('get_power_modes')});
 async function savePolicy(){await invoke('save_power_settings',{settings:policy})}
 useDraft(()=>policy,savePolicy,()=>policy!==null);
 async function setMode(profile:string){if(modeBusy)return;modeBusy=true;modeError='';try{await invoke('set_power_mode',{profile});modes=await invoke('get_power_modes')}catch(e){modeError=String(e)}finally{modeBusy=false}}
 const timerOptions=['0','60','120','300','480','510','600','900','1200','1800','3600'];
 // Any seconds value reads as words, including ones written outside these presets
 // (hypridle.conf had 240 and 330, which the select showed as raw numbers).
 function timerLabel(v:string):string{const n=Number(v);if(!n)return 'Never';const h=Math.floor(n/3600),m=Math.floor(n%3600/60),sec=n%60;const part=(x:number,u:string)=>`${x} ${u}${x===1?'':'s'}`;return [h&&part(h,'hour'),m&&part(m,'minute'),sec&&part(sec,'second')].filter(Boolean).join(' ');}
 const timerLabels=(values:string[])=>Object.fromEntries(values.map(v=>[v,timerLabel(v)]));
 let history=$state<{time:number;percentage:number}[]>([]);let historyError=$state('');
 onMount(async()=>{try{history=await invoke("get_battery_history")}catch(e){historyError=String(e)}});
 let chart=$derived(history.length>1 ? history.map((s,i)=>`${i===0?"M":"L"}${(s.time-history[0].time)/(history[history.length-1].time-history[0].time || 1)*200} ${48-s.percentage*.48}`).join(" ") : "");
 let info = $state<PowerInfo | null>(null);
	let brightness = $state(0);
	let chargeStart = $state(0);
	let chargeEnd = $state(100);

	// Brightness keys, the bar and other tools change the backlight (and the
	// battery drains) while this page is open, so poll while it is visible
	// (BSET-31). A read never replaces a slider that is being dragged or was
	// just moved.
	const gate = createLiveGate();
	let ticks = 0;

	onMount(() => {
		(async () => {
			info = await invoke<PowerInfo>("get_power_info");
			brightness = info.brightness_pct ?? 0;
			chargeStart = info.charge_start ?? 0;
			chargeEnd = info.charge_end ?? 100;
		})();
		return pollWhileVisible(async () => {
			ticks += 1;
			if (!info) return;
			const b = await invoke<number | null>("get_brightness");
			if (b !== null && gate.canAdopt()) brightness = b;
			// Battery level and power source change slowly: every ~10 s.
			if (ticks % 5 === 0) {
				const fresh = await invoke<PowerInfo>("get_power_info");
				if (info) {
					info.battery = fresh.battery;
					info.power_source = fresh.power_source;
				}
			}
		}, 2000);
	});

	async function setBrightness(percent: number) {
		gate.touch();
		brightness = percent;
		await invoke("set_brightness", { percent });
	}

	const PROTECT_STOP = 80;
	const PROTECT_RESUME = 75;
	let protectMsg = $state("");

	// One switch over whichever mechanism the hardware has (thresholds,
	// charge_types Long_Life, or ideapad conservation_mode). The switch only
	// moves once the privileged write succeeded; a refused or cancelled
	// authentication leaves it where it was and says why.
	async function setProtection(on: boolean) {
		if (!info?.protection) return;
		protectMsg = "";
		const before = info.protection.enabled;
		info.protection.enabled = on;
		try {
			info.protection = await invoke<BatteryProtection>("set_battery_protection", { enabled: on });
			if (info.protection.limit_pct !== null) chargeEnd = info.protection.limit_pct;
		} catch (e) {
			info.protection.enabled = before;
			protectMsg = `${e}`;
		}
	}

	async function setChargeThreshold(which: "start" | "end", percent: number) {
		if (which === "start") chargeStart = percent;
		else chargeEnd = percent;
		await invoke("set_charge_threshold", { which, percent });
	}
</script>

<ViewScaffold
	title="Power & battery"
	lede="Brightness, battery, and charge limits."
>
 {#if info}
 <div class="battery-hero"><div class="ring">{info.battery.find(([k])=>k==='Charge')?.[1] ?? 'AC'}</div><div><h2>{info.battery.find(([k])=>k==='Time remaining')?.[1] ?? info.power_source}</h2><p>{info.power_source} · {info.battery.find(([k])=>k==='Battery health')?.[1] ?? ''} · {info.battery.find(([k])=>k==='Power draw')?.[1] ?? ''}</p></div>{#if chart}<svg viewBox="0 0 200 48" aria-label="Battery level, last 24 hours"><path d={chart} fill="none" stroke="var(--accent)" stroke-width="1.6"/></svg>{:else}<small>{historyError || "Battery history is not available yet."}</small>{/if}</div>
 <Group title="Performance">
 {#if modes?.profiles.length}<Row label="Power mode"><div class="seg">{#each modes.profiles as profile}<button class="btn" class:primary={modes.current===profile} disabled={modeBusy||automatic} onclick={()=>setMode(profile)}>{{'power-saver':'Battery saver','balanced':'Balanced','performance':'Performance'}[profile] ?? profile}</button>{/each}</div></Row>{:else if modes?.tlp_setup_available}<Row label="Power mode" hint="TLP is installed. Power modes need its small profile service (tlp-pd). Needs your password."><button class="btn primary" disabled={modeBusy} onclick={setupModes}>{modeBusy ? "Setting up…" : "Set up power modes"}</button></Row>{#if setupLog.length}<LogView lines={setupLog}/>{/if}{:else}<Hint text="Power modes need a power profile service. None is installed."/>{/if}
 <SwitchField disabled={modeBusy||!modes?.profiles.includes("power-saver")||!modes?.profiles.includes("balanced")||!modes?.profiles.includes("performance")} label="Switch automatically" hint="Performance on AC, Balanced on battery, Battery saver below 20%." bind:value={()=>automatic,v=>{void setAutomatic(v)}}/>
 {#if modeError}<p role="alert">{modeError}</p>{/if}
 </Group>
 {#if policy}
 <Group title="When you step away">
 {#each [['dim','Dim the screen after'],['screen_off','Turn off the screen after'],['lock','Lock after'],['sleep','Sleep after']] as [key,label]}
 <SelectField {label} bind:value={()=>String(policy![key as 'dim']),v=>policy![key as 'dim']=Number(v)} options={Array.from(new Set([...timerOptions,String(policy[key as 'dim'])])).sort((a,b)=>Number(a)-Number(b))} labels={timerLabels([...timerOptions,String(policy[key as 'dim'])])}/>
 {/each}
 <SwitchField label="Sleep only on battery" bind:value={policy.sleep_on_battery}/>
 </Group>
 <Group title="Buttons and lid" hint="Changes to these two settings take effect after restarting the computer.">
 <SelectField label="When I close the lid" bind:value={policy.lid} options={['suspend','lock','ignore']} labels={{suspend:'Sleep',lock:'Lock',ignore:'Do nothing'}}/>
 <SelectField label="Power button" bind:value={policy.power_button} options={['menu','suspend','poweroff','ignore']} labels={{menu:'Show menu',suspend:'Sleep',poweroff:'Power off',ignore:'Do nothing'}}/>
 </Group><SaveButton onSave={savePolicy}/>
 {/if}
		<Group title="Battery">
			{#each info.battery as [label, value] (label)}
				<InfoRow {label} {value} />
			{/each}
			<InfoRow label="Power source" value={info.power_source} />
		</Group>

		<Group title="Brightness">
			{#if info.brightness_pct !== null}
				<Row label="Screen brightness">
					<input
						type="range"
						min="1"
						max="100"
						bind:value={brightness}
						onpointerdown={() => gate.begin()}
						onpointerup={() => gate.end()}
						onpointercancel={() => gate.end()}
						onchange={() => setBrightness(brightness)}
					/>
					<span class="pct">{brightness}%</span>
				</Row>
			{:else}
				<Hint text="No controllable backlight found." />
			{/if}
		</Group>

		{#if info.protection}
			<Group
				title="Battery protection"
				hint={info.protection.kind === "thresholds"
					? `Stops charging at ${PROTECT_STOP}% and resumes at ${PROTECT_RESUME}%. Needs your password.`
					: "Stops charging at about 80% to make the battery last longer. Needs your password."}
			>
				<SwitchField
					label="Protect battery"
					hint={info.protection.enabled ? "On: charging is limited." : "Off: charges to 100%."}
					bind:value={() => info!.protection!.enabled, (v) => setProtection(v)}
				/>
				{#if protectMsg}<Hint text={protectMsg} />{/if}
			</Group>
		{/if}

		{#if info.charge_start !== null && info.charge_end !== null}
			<Group
				title="Charge limits"
				hint="Keeps the battery off 100% when this machine stays plugged in."
			>
				<Row label="Start charging below">
					<input type="range" min="0" max="100" bind:value={chargeStart} onchange={() => setChargeThreshold("start", chargeStart)} />
					<span class="pct">{chargeStart}%</span>
				</Row>
				<Row label="Stop charging at">
					<input type="range" min="1" max="100" bind:value={chargeEnd} onchange={() => setChargeThreshold("end", chargeEnd)} />
					<span class="pct">{chargeEnd}%</span>
				</Row>
			</Group>
		{/if}

		<Group
			title="TLP"
			hint="TLP picks battery vs AC on its own."
		>
			<InfoRow label="Current profile" value={info.tlp_profile ?? "unknown"} />
		</Group>
	{/if}
</ViewScaffold>

<style>
 .battery-hero{display:flex;align-items:center;gap:16px;padding:18px;border-radius:14px;background:var(--surface);border:1px solid var(--line)}.ring{border:4px solid var(--accent);border-radius:50%;width:66px;height:66px;display:grid;place-items:center;flex:none}.battery-hero svg{width:220px;height:52px;margin-left:auto;flex-shrink:0}.battery-hero{flex-wrap:wrap}.battery-hero small{font-size:11px;color:var(--muted)}.battery-hero h2{font-size:18px;font-weight:500;margin:0}.battery-hero p{color:var(--muted);font-size:12px;margin:4px 0 0}.seg{display:flex;flex-wrap:wrap;gap:3px}
	input[type="range"] {
		width: 180px;
	}

	.pct {
		margin-left: var(--space-sm, 8px);
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.7;
	}
</style>
