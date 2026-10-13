<script lang="ts">
	import {useDraft} from "$lib/drafts.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { runStreamed } from "$lib/streaming";
	import LogView from "$lib/components/LogView.svelte";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
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

	const MODE_LABELS: Record<string, string> = { "power-saver": "Battery saver", balanced: "Balanced", performance: "Performance" };
	const field = (k: string) => info?.battery.find(([label]) => label === k)?.[1] ?? null;
	let charge = $derived.by(() => { const v = parseInt(field("Charge") ?? "", 10); return Number.isFinite(v) ? v : null; });
	// "8.1 hours" / "45 minutes" from the backend -> "8 h 6 m".
	function duration(text: string | null): string | null {
		const m = text?.match(/([\d.]+)\s*(hour|minute)/);
		if (!m) return null;
		const mins = Math.round(Number(m[1]) * (m[2] === "hour" ? 60 : 1));
		return mins >= 60 ? `${Math.floor(mins / 60)} h ${mins % 60} m` : `${mins} m`;
	}
	let headline = $derived.by(() => {
		if (!info) return "";
		const status = field("Status");
		const left = duration(field("Time remaining"));
		const full = duration(field("Time until full"));
		if (left) return `${left} left`;
		if (full) return `Full in ${full}`;
		if (charge === null) return "Plugged in";
		return status ?? info.power_source;
	});
	let detail = $derived.by(() => {
		if (!info) return "";
		const draw = parseFloat(field("Power draw") ?? "");
		const status = field("Status") ?? info.power_source;
		const parts = status === headline ? [] : [status];
		if (Number.isFinite(draw) && draw > 0) parts.push(`drawing ${draw.toFixed(1)} W`);
		const health = field("Battery health");
		if (health) parts.push(`battery health ${health.replace(" of design capacity", "")}`);
		return parts.join(" · ") || info.power_source;
	});

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
	lede="Battery, performance, and what happens when you step away."
>
	{#if info}
		<div class="hero-card power-hero">
			{#if charge !== null}<span class="ring" style="--v:{charge}">{charge}%</span>{/if}
			<div>
				<h3>{headline}</h3>
				<p>{detail}</p>
			</div>
			{#if chart}<svg viewBox="0 0 200 48" aria-label="Battery level, last 24 hours"><path d="{chart} L200 48 L0 48Z" fill="var(--accent-soft)" stroke="none"/><path d={chart} fill="none" stroke="var(--accent)" stroke-width="1.6"/></svg>{/if}
		</div>
	{/if}
	<Group title="Performance">
		{#if modes?.profiles.length}
			<Row label="Power mode" hint={automatic ? "Chosen automatically right now." : undefined}>
				<div class="ui-seg">{#each modes.profiles as profile (profile)}<button class:on={modes.current === profile} disabled={modeBusy || automatic} onclick={() => setMode(profile)}>{MODE_LABELS[profile] ?? profile}</button>{/each}</div>
			</Row>
		{:else if modes?.tlp_setup_available}
			<Row label="Power mode" hint="Lets you pick Battery saver, Balanced or Performance. Installs a small helper for TLP and needs your password.">
				<button class="btn primary small" disabled={modeBusy} onclick={setupModes}>{modeBusy ? "Setting up…" : "Set up power modes"}</button>
			</Row>
			{#if setupLog.length}<LogView lines={setupLog} />{/if}
		{:else}
			<Hint text="Power modes need a power profile service, and none is installed." />
		{/if}
		{#if modes?.profiles.length}
			<SwitchField disabled={modeBusy || !["power-saver", "balanced", "performance"].every((p) => modes?.profiles.includes(p))} label="Switch automatically" hint="Performance when plugged in, Balanced on battery, Battery saver below 20%." bind:value={() => automatic, (v) => { void setAutomatic(v); }} />
		{/if}
		{#if modeError}<p class="err" role="alert">{modeError}</p>{/if}
	</Group>
	{#if info}
		{#if info.brightness_pct !== null}
			<Group title="Screen">
				<Row label="Brightness">
					<input type="range" min="1" max="100" bind:value={brightness} onpointerdown={() => gate.begin()} onpointerup={() => gate.end()} onpointercancel={() => gate.end()} onchange={() => setBrightness(brightness)} />
					<span class="pct">{brightness}%</span>
				</Row>
			</Group>
		{/if}
		{#if info.protection || (info.charge_start !== null && info.charge_end !== null)}
			<Group title="Battery">
				{#if info.protection}
					<SwitchField
						label="Protect battery"
						hint={info.protection.kind === "thresholds" ? `Stops charging at ${PROTECT_STOP}% and starts again below ${PROTECT_RESUME}%, so the battery lasts longer. Needs your password.` : "Stops charging at about 80% so the battery lasts longer. Needs your password."}
						bind:value={() => info!.protection!.enabled, (v) => setProtection(v)}
					/>
					{#if protectMsg}<p class="err" role="alert">{protectMsg}</p>{/if}
				{/if}
				{#if info.charge_start !== null && info.charge_end !== null}
					<details class="ui-disclosure">
						<summary>Exact charge limits</summary>
						<Row label="Start charging below">
							<input type="range" min="0" max="100" bind:value={chargeStart} onchange={() => setChargeThreshold("start", chargeStart)} />
							<span class="pct">{chargeStart}%</span>
						</Row>
						<Row label="Stop charging at">
							<input type="range" min="1" max="100" bind:value={chargeEnd} onchange={() => setChargeThreshold("end", chargeEnd)} />
							<span class="pct">{chargeEnd}%</span>
						</Row>
					</details>
				{/if}
			</Group>
		{/if}
	{/if}
	{#if policy}
		<Group title="When you step away">
			{#each [["dim", "Dim the screen after"], ["screen_off", "Turn off the screen after"], ["lock", "Lock after"], ["sleep", "Sleep after"]] as [key, label] (key)}
				<SelectField {label} bind:value={() => String(policy![key as "dim"]), (v) => (policy![key as "dim"] = Number(v))} options={Array.from(new Set([...timerOptions, String(policy[key as "dim"])])).sort((a, b) => Number(a) - Number(b))} labels={timerLabels([...timerOptions, String(policy[key as "dim"])])} />
			{/each}
			<SwitchField label="Sleep only on battery" hint="Stay awake while plugged in." bind:value={policy.sleep_on_battery} />
		</Group>
		<Group title="Buttons and lid" hint="These two take effect after you restart the computer.">
			<Row label="When I close the lid">
				<div class="ui-seg">{#each [["suspend", "Sleep"], ["lock", "Lock"], ["ignore", "Do nothing"]] as [v, l] (v)}<button class:on={policy.lid === v} onclick={() => (policy!.lid = v)}>{l}</button>{/each}</div>
			</Row>
			<Row label="Power button">
				<div class="ui-seg">{#each [["menu", "Show menu"], ["suspend", "Sleep"], ["poweroff", "Power off"], ["ignore", "Do nothing"]] as [v, l] (v)}<button class:on={policy.power_button === v} onclick={() => (policy!.power_button = v)}>{l}</button>{/each}</div>
			</Row>
		</Group>
	{/if}
</ViewScaffold>

<style>
	.power-hero { flex-wrap:wrap; }
	.power-hero svg { width:220px; height:52px; margin-left:auto; flex-shrink:0; }
	.ring { --v:0; width:62px; height:62px; flex:none; border-radius:50%; display:grid; place-items:center; font-size:13px; background:radial-gradient(closest-side, var(--surface) 80%, transparent 81% 100%), conic-gradient(var(--accent) calc(var(--v) * 1%), var(--surface-2) 0); }
	input[type="range"] { width:180px; }
	.pct { margin-left:8px; font-size:12px; color:var(--muted); min-width:3ch; }
	.err { color:var(--red); font-size:12.5px; margin:4px 2px 10px; }
</style>
