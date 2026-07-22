<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import Group from "$lib/components/Group.svelte";

	interface SystemInfo {
		os: string;
		kernel: string;
		cpu: string;
		gpu: string;
		memory: string;
		disk: string;
		uptime: string;
		hostname: string;
	}

	let info = $state<SystemInfo | null>(null);
	let hostnameInput = $state("");
	let status = $state("");
	let applying = $state(false);

	// The backend reports the GPU as a raw lspci device string, e.g.
	// "Advanced Micro Devices, Inc. [AMD/ATI] Krackan [Radeon 840M / 860M Graphics] (rev c2)".
	// That's too technical for a general settings page, so extract just the
	// marketing model name. Heuristic (not exhaustive — just needs to read
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
		hostnameInput = info.hostname;
	});

	async function applyHostname() {
		if (!hostnameInput.trim()) {
			status = "Hostname can't be empty";
			return;
		}
		applying = true;
		status = "Applying…";
		try {
			await invoke("set_hostname", { name: hostnameInput.trim() });
			status = "Applied";
		} catch (e) {
			status = `Error: ${e}`;
		} finally {
			applying = false;
		}
	}
</script>

<ViewScaffold title="About">
	{#if info}
		<Group title="This machine">
			<InfoRow label="Operating system" value={info.os} />
			<InfoRow label="Kernel" value={info.kernel} />
			<InfoRow label="CPU" value={info.cpu} />
			<InfoRow label="GPU" value={friendlyGpu(info.gpu)} />
			<InfoRow label="Memory" value={info.memory} />
			<InfoRow label="Disk (/)" value={info.disk} />
			<InfoRow label="Uptime" value={info.uptime} />
		</Group>

		<Group title="Hostname" hint="Changes the machine's network name. Takes effect immediately; needs your password (polkit).">
			<div class="hostname-row">
				<input type="text" bind:value={hostnameInput} disabled={applying} />
				<button disabled={applying} onclick={applyHostname}>Apply</button>
			</div>
			{#if status}
				<span class="status">{status}</span>
			{/if}
		</Group>
	{/if}
</ViewScaffold>

<style>
	.hostname-row {
		display: flex;
		gap: var(--space-md, 12px);
		flex: 1;
	}

	input {
		flex: 1;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		background-color: var(--accent);
		color: var(--on-accent);
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-sm, 8px) var(--space-lg, 16px);
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.status {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}
</style>
