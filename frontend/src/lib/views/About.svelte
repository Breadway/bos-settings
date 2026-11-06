<script lang="ts">
	import Row from "$lib/components/Row.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Group from "$lib/components/Group.svelte";
	import { go } from "$lib/nav.svelte";

	interface SystemInfo {
		os: string;
        version:string|null;
		kernel: string;
		cpu: string;
		gpu: string;
		memory: string;
		disk: string;
		uptime: string;
		hostname: string;
	}

	type HealthState = "ok" | "degraded" | "failed";
	let checks=$state<{name:string;status:HealthState;ok:boolean;summary:string;output:string}[]>([]);
	let checking=$state(false);
	async function health(){checking=true;status="";try{checks=await invoke("run_health_checks")}catch(e){status=String(e)}finally{checking=false}}
	let track = $state("");
	let info = $state<SystemInfo | null>(null);
	let status = $state("");

	// The backend reports the GPU as a raw lspci device string, e.g.
	// "Advanced Micro Devices, Inc. [AMD/ATI] Krackan [Radeon 840M / 860M Graphics] (rev c2)".
	// That's too technical for a general settings page, so extract just the
	// marketing model name. Heuristic (not exhaustive  -  just needs to read
	// well for common vendors): drop the trailing "(rev ..)", prefer the
	// text inside the last [...] bracket group (usually the model name),
	// collapse "840M / 860M" style multi-model lists to the last variant,
	// and drop a trailing generic word like "Graphics"/"Controller".
	function friendlyGpu(raw: string): string {
		if (!raw) return raw;
		let s = raw.replace(/\s*\(rev [^)]*\)\s*$/i, "").trim();
		const brackets = [...s.matchAll(/\[([^\]]+)\]/g)];
		if (brackets.length > 0) {
			s = brackets[brackets.length - 1][1];
		} else {
			s = s.replace(/^.*?,\s*Inc\.\s*/i, "").trim();
		}
		s = s.replace(/\b(\w+)\s+[\w.]+\s*\/\s*(\w+)\b/, "$1 $2");
		s = s.replace(/\s+(Graphics|Controller|Series)\s*$/i, "");
		return s.trim() || raw;
	}

	onMount(async () => {
		info = await invoke<SystemInfo>("get_system_info");
		track = await invoke<{current:string}>("get_bakery_track").then(result => result.current).catch(() => "Unavailable");
	});

	async function copyInfo() {
		try {
			await navigator.clipboard.writeText(`${info!.os}${info!.version ? ` ${info!.version}` : ""}\nKernel: ${info!.kernel}\nCPU: ${info!.cpu}\nGPU: ${info!.gpu}\nMemory: ${info!.memory}\nChannel: ${track}`);
			status = "System info copied.";
		} catch (e) {
			status = String(e);
		}
	}
</script>

<ViewScaffold title="About">
	{#if info}
		<div class="hero-card">
			<span class="big">B</span>
			<div><h3>{info.os}{info.version ? ` ${info.version}` : ""}</h3><p>{track ? `${track.charAt(0).toUpperCase()}${track.slice(1)} channel · ` : ""}{info.uptime}</p></div>
		</div>
		<Group title="This computer">
			<InfoRow label="Processor" value={info.cpu} />
			<InfoRow label="Graphics" value={friendlyGpu(info.gpu)} />
			<InfoRow label="Memory" value={info.memory} />
			<Row label="Storage" hint={info.disk}><button class="btn small ghost" onclick={() => go("storage")}>Manage</button></Row>
			<Row label="Name" hint={info.hostname}><button class="btn small ghost" onclick={() => go("sharing")}>Change</button></Row>
			<details class="ui-disclosure">
				<summary>Technical details</summary>
				<InfoRow label="Operating system" value={info.os} />
				{#if info.version}<InfoRow label="Version" value={info.version} />{/if}
				<InfoRow label="Kernel" value={info.kernel} />
				<InfoRow label="Graphics (full name)" value={info.gpu} />
			</details>
		</Group>
		<Group title="Help">
			<Row label="Copy system info" hint="Paste it into a bug report."><button class="btn small" onclick={copyInfo}>Copy</button></Row>
			<Row label="Run a health check" hint="Checks the bread daemon and adapters, systemd user services, and package tools."><button class="btn small" disabled={checking} onclick={health}>{checking ? "Checking…" : "Run"}</button></Row>
			{#each checks as check (check.name)}
				<details class="check">
					<summary><span>{check.name}</span><span class="ui-tag" class:ok={check.status === "ok"} class:warn={check.status === "degraded"} class:bad={check.status === "failed"}>{check.status === "ok" ? "OK" : check.status === "degraded" ? "Degraded" : "Failed"}</span></summary>
					{#if check.summary}<p class="summary">{check.summary}</p>{/if}
					<pre>{check.output}</pre>
				</details>
			{/each}
			{#if status}<p class="msg" role="status">{status}</p>{/if}
		</Group>
	{/if}
</ViewScaffold>

<style>
	.hero-card .big { font:500 24px var(--font-display, 'Outfit', sans-serif); }
	.check summary { display:flex; justify-content:space-between; align-items:center; padding:9px 2px; cursor:pointer; font-size:13px; }
	.check pre { font-size:11.5px; color:var(--muted); white-space:pre-wrap; margin:0 2px 10px; }
	.check .summary { margin:0 2px 6px; font-size:12.5px; color:var(--fg); }
	.ui-tag.warn { background:rgba(214,178,94,.14); color:#d6b25e; }
	.ui-tag.bad { background:rgba(214,94,94,.14); color:#d65e5e; }
	.msg { margin:4px 2px 10px; color:var(--muted); font-size:12.5px; }
</style>
