<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount, onDestroy } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import Row from "$lib/components/Row.svelte";
	import InfoRow from "$lib/components/InfoRow.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import TagsField from "$lib/components/TagsField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";

	const ACTION_LABELS: Record<string, string> = {
		cgroup_soft_limit: "Soft memory limits",
		reclaim_protection: "Reclaim protection",
		unload_idle_model: "Unload idle models",
		sleep_model: "Sleep model backends",
		reduce_job_concurrency: "Reduce job workers",
		queue_background_job: "Delay background jobs",
	};

	const PROFILES: { id: string; label: string; hint: string }[] = [
		{ id: "monitor", label: "Monitor", hint: "Watch only. Nothing ever acts." },
		{ id: "desktop", label: "Desktop", hint: "Automatic relief that leaves interactive apps alone." },
		{ id: "headless-server", label: "Headless server", hint: "Automatic relief that leans on background jobs first." },
	];

	interface ProviderEntry {
		name: string;
		kind: string;
		url: string;
		scope: string;
		allow_automation: boolean;
	}

	interface BreadramFile {
		mode: string | null;
		allowed_actions: string[];
		protected: string[];
		providers: ProviderEntry[];
		jobs_enabled: boolean | null;
		jobs_total_budget_gib: number | null;
		jobs_default_estimate_gib: number | null;
		jobs_auto_launch: boolean | null;
	}

	interface BreadramConfig {
		mode: string;
		allowed_actions: string[];
		protected: string[];
		providers: ProviderEntry[];
		jobs_enabled: boolean;
		jobs_total_budget_gib: number;
		jobs_default_estimate_gib: number;
		jobs_auto_launch: boolean;
	}

	interface CapabilityEntry {
		name: string;
		state: string;
		reason?: string | null;
	}

	interface LiveStatus {
		present: boolean;
		mode?: string;
		version?: string;
		pressure?: { state?: string; some_avg10?: number | null };
		memory?: {
			ram_available_bytes?: number | null;
			ram_total_bytes?: number | null;
			vram_bytes?: number | null;
			gtt_used_bytes?: number | null;
			gtt_total_bytes?: number | null;
		};
		capabilities?: { capabilities?: CapabilityEntry[] };
		journal?: { entries?: number; capacity?: number; executing?: boolean };
		config_path?: string | null;
		notes?: string[];
		rules?: {
			mode?: string;
			allowed_actions?: string[];
			protected?: string[];
			budgets?: { scope: string; max_bytes: number }[];
			rules?: { name: string }[];
		} | null;
	}

	interface ActionRecord {
		scope: string;
		kind: string | null;
		state: string;
		reason: string;
		requested_host_ram_delta?: number | null;
		observed_host_ram_delta?: number | null;
	}

	interface LiveActions {
		present: boolean;
		actions?: {
			plan?: { state?: string; would_execute?: boolean; explanation?: string[] };
			history?: ActionRecord[];
		} | null;
	}

	const RELOAD_NOTES: Record<string, string> = {
		ollama: "Release keeps the model unloaded until something asks for it; the next request pays a cold load.",
		llamacpp: "Unload drops the model; the router reloads it on the next request.",
		vllm: "Sleep keeps weights staged and wakes faster than a cold load.",
		breadsearch: "Unload drops the embedding session; the next search reloads it on demand.",
		hook: "Release and reload mean whatever the hook program does.",
	};

	let cfg = $state<BreadramConfig | null>(null);
	let status = $state<LiveStatus | null>(null);
	let actions = $state<LiveActions | null>(null);
	let liveError = $state("");
	let modeBusy = $state(false);
	let profileBusy = $state("");

	async function loadFile() {
		const loaded = await invoke<BreadramFile>("get_breadram_config");
		// Normalize missing keys to the daemon's own defaults so every
		// control binds a real value. Saving then states explicitly what
		// was implicit, which the effective-settings section shows back.
		cfg = {
			mode: loaded.mode ?? "monitor",
			allowed_actions: loaded.allowed_actions,
			protected: loaded.protected,
			providers: loaded.providers,
			jobs_enabled: loaded.jobs_enabled ?? false,
			jobs_total_budget_gib: loaded.jobs_total_budget_gib ?? 8,
			jobs_default_estimate_gib: loaded.jobs_default_estimate_gib ?? 2,
			jobs_auto_launch: loaded.jobs_auto_launch ?? false,
		};
	}

	async function refreshLive() {
		try {
			const [s, a] = await Promise.all([
				invoke<LiveStatus>("breadram_status"),
				invoke<LiveActions>("breadram_actions"),
			]);
			status = s;
			actions = a;
			liveError = "";
		} catch (e) {
			liveError = String(e);
		}
	}

	let timer: ReturnType<typeof setInterval> | undefined;

	onMount(async () => {
		await loadFile();
		await refreshLive();
		timer = setInterval(refreshLive, 10_000);
	});

	onDestroy(() => clearInterval(timer));

	async function save() {
		await invoke("save_breadram_config", { cfg });
		await refreshLive();
	}
	useDraft(() => (cfg), save, () => cfg !== null);

	async function pauseNow(paused: boolean) {
		modeBusy = true;
		try {
			await invoke("set_breadram_mode", { mode: paused ? "monitor" : "automatic", persist: false });
			await refreshLive();
		} catch (e) {
			liveError = String(e);
		}
		modeBusy = false;
	}

	async function applyProfile(id: string) {
		profileBusy = id;
		try {
			await invoke("apply_breadram_profile", { profile: id });
			await loadFile();
			await refreshLive();
		} catch (e) {
			liveError = String(e);
		}
		profileBusy = "";
	}

	function gib(bytes: number | null | undefined): string {
		if (bytes == null) return "unknown";
		if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
		if (bytes >= 1024 ** 2) return `${Math.round(bytes / 1024 ** 2)} MiB`;
		return `${Math.round(bytes / 1024)} KiB`;
	}

	function deltaText(delta: number | null | undefined): string {
		if (delta == null) return "not measured";
		if (delta < 0) return `freed ${gib(-delta)}`;
		return `used ${gib(delta)}`;
	}

	function addProvider() {
		if (!cfg) return;
		cfg.providers = [
			...cfg.providers,
			{ name: "", kind: "ollama", url: "", scope: "", allow_automation: false },
		];
	}
</script>

<ViewScaffold title="Memory">
	<Group
		title="Right now"
		hint="What the memory daemon is doing. Nothing here needs the daemon running to open this page."
	>
		{#if status && !status.present}
			<Row label="Daemon" hint="breadram is not running. Settings below still open and save.">
				<span>Unavailable</span>
			</Row>
		{:else if status}
			<InfoRow label="Pressure" value={String(status.pressure?.state ?? "unknown")} />
			<InfoRow label="Mode" value={String(status.mode ?? "unknown")} />
			<InfoRow
				label="Available"
				value={`${gib(status.memory?.ram_available_bytes)} of ${gib(status.memory?.ram_total_bytes)}`}
			/>
			<InfoRow
				label="Device memory"
				value={status.memory?.vram_bytes != null
					? gib(status.memory.vram_bytes)
					: status.memory?.gtt_used_bytes != null
						? `${gib(status.memory.gtt_used_bytes)} shared of ${gib(status.memory.gtt_total_bytes)}`
						: "nothing measured it"}
			/>
			<InfoRow label="Daemon version" value={String(status.version ?? "unknown")} />
			<Row label="Pause" hint="Stops automatic changes now without saving anything. A restart goes back to the saved mode.">
				<button
					onclick={() => pauseNow(status?.mode !== "monitor")}
					disabled={modeBusy || !status?.present}
				>
					{status?.mode === "monitor" ? "Resume" : "Pause"}
				</button>
			</Row>
			{#if liveError}
				<Row label="Live update failed" hint={liveError}>
					<button onclick={refreshLive}>Retry</button>
				</Row>
			{/if}
		{:else}
			<Row label="Daemon" hint="Reading the daemon for the first time.">
				<span>Loading…</span>
			</Row>
		{/if}
	</Group>

	{#if cfg}
		<Group
			title="Starting point"
			hint="Named setups that fill in the settings below. You can still change anything afterwards."
		>
			<Row label="Profile" hint="Monitor watches only. The other two act automatically with different caution.">
				<span class="profile-row">
					{#each PROFILES as profile (profile.id)}
						<button
							onclick={() => applyProfile(profile.id)}
							disabled={profileBusy !== ""}
							title={profile.hint}
						>
							{profileBusy === profile.id ? "Applying…" : profile.label}
						</button>
					{/each}
				</span>
			</Row>
		</Group>

		<Group
			title="Automation"
			hint="What the daemon may do on its own. Unticked actions never run automatically."
		>
			<Row label="Mode" hint="Monitor watches and explains. Automatic acts within the ticked actions.">
				<select bind:value={cfg.mode}>
					<option value="monitor">Monitor</option>
					<option value="automatic">Automatic</option>
				</select>
			</Row>
			{#each Object.entries(ACTION_LABELS) as [code, label] (code)}
				{@const on = cfg.allowed_actions.includes(code)}
				{@const enforced = status && status.present && status.rules
					? status.rules.allowed_actions?.includes(code) ?? false
					: null}
				<Row
					{label}
					hint={on
						? enforced === false
							? "Ticked in the file, but the running daemon has it off."
							: "Allowed."
						: "Off. The daemon will not do this."}
				>
					<input
						type="checkbox"
						checked={on}
						aria-label={label}
						onchange={(e) => {
							const checked = (e.target as HTMLInputElement).checked;
							cfg!.allowed_actions = checked
								? [...cfg!.allowed_actions, code]
								: cfg!.allowed_actions.filter((a) => a !== code);
						}}
					/>
				</Row>
			{/each}
			<TagsField label="Never touch" bind:value={cfg.protected} />
		</Group>

		<Group
			title="Model backends"
			hint="Extra programs the daemon may ask to release memory. Each one only watches the workload scope you name."
		>
			{#each cfg.providers as provider, i (i)}
				<Row
					label={provider.name || `Backend ${i + 1}`}
					hint={`${provider.url}${RELOAD_NOTES[provider.kind] ? ` ${RELOAD_NOTES[provider.kind]}` : ""}`}
				>
					<span class="provider-row">
						<TextField label="Name" bind:value={provider.name} />
						<TextField label="Address" bind:value={provider.url} />
						<TextField label="Workload scope" bind:value={provider.scope} />
						<select bind:value={provider.kind} aria-label="Backend kind">
							<option value="ollama">Ollama</option>
							<option value="llamacpp">llama.cpp</option>
							<option value="vllm">vLLM</option>
							<option value="breadsearch">Breadsearch</option>
							<option value="hook">Custom hook</option>
						</select>
						<label>
							<input type="checkbox" bind:checked={provider.allow_automation} />
						 May act
						</label>
						<button
							onclick={() => {
								cfg!.providers = cfg!.providers.filter((_, j) => j !== i);
							}}
						>
							Remove
						</button>
					</span>
				</Row>
			{/each}
			<Row label="Backends" hint="Ollama and llama.cpp speak HTTP. Breadsearch uses its socket.">
				<button onclick={addProvider}>Add backend</button>
			</Row>
		</Group>

		<Group
			title="Background jobs"
			hint="Queued programs the daemon may run when memory allows. breadram never caps a workload by default: only a supervised job carries a hard MemoryMax reservation, taken from its estimate."
		>
			<SwitchField label="Let the daemon run queued jobs" bind:value={cfg.jobs_enabled} />
			<NumberField label="Total memory for jobs (GiB)" bind:value={cfg.jobs_total_budget_gib} min={0.1} max={1024} step={0.5} />
			<NumberField
				label="Guess per job (GiB)"
				bind:value={cfg.jobs_default_estimate_gib}
				min={0.1}
				max={1024}
				step={0.5}
			/>
			<SwitchField label="Start the next job on its own" bind:value={cfg.jobs_auto_launch} />
		</Group>

		<Group
			title="What the daemon thinks"
			hint="The settings actually in force, and why. File changes apply when the daemon restarts."
		>
			{#if status && status.present && status.rules}
				<InfoRow label="Mode in force" value={String(status.rules.mode ?? "unknown")} />
				<InfoRow
					label="Actions in force"
					value={(status.rules.allowed_actions ?? []).join(", ") || "none"}
				/>
				<InfoRow
					label="Protected in force"
					value={(status.rules.protected ?? []).join(", ") || "none"}
				/>
				{#each status.rules.budgets ?? [] as budget (budget.scope)}
					<InfoRow label={`Budget ${budget.scope}`} value={gib(budget.max_bytes)} />
				{/each}
				<InfoRow label="Config file" value={String(status.config_path ?? "none loaded")} />
			{:else}
				<Row label="Effective settings" hint="Start the daemon to compare the file against what is running.">
					<span>Daemon unavailable</span>
				</Row>
			{/if}
			<ServiceControl unit="breadram.service" name="Memory daemon" bare hasConfig />
		</Group>

		<Group
			title="What this host can do"
			hint="Each capability the daemon probed, and why anything unavailable stays off. A workload is observation-only when its controls are missing here."
		>
			{#if status && status.present && (status.capabilities?.capabilities?.length ?? 0) > 0}
				{#each status.capabilities?.capabilities ?? [] as capability (capability.name)}
					<InfoRow
						label={capability.name}
						value={capability.state === "available"
							? "Available"
							: `${capability.state}${capability.reason ? `: ${capability.reason}` : ""}`}
					/>
				{/each}
			{:else}
				<Row label="Capabilities" hint="Start the daemon to probe this host.">
					<span>Daemon unavailable</span>
				</Row>
			{/if}
		</Group>

		<Group title="Recent changes" hint="Newest first. Expected is the plan's guess, measured is what the daemon saw.">
			{#if actions && actions.present && actions.actions?.plan}
				<InfoRow
					label={`Plan (${actions.actions.plan.state ?? "unknown"})`}
					value={(actions.actions.plan.explanation ?? []).slice(0, 3).join(" " ) || "Nothing proposed."}
				/>
			{/if}
			{#if actions && actions.present && actions.actions && (actions.actions.history?.length ?? 0) > 0}
				{#each (actions.actions.history ?? []).slice(0, 20) as record, n (n)}
					<Row
						label={`${record.kind ?? "change"} · ${record.scope}`}
						hint={record.reason}
					>
						<span>{record.state} · expected {deltaText(record.requested_host_ram_delta)}, measured {deltaText(record.observed_host_ram_delta)}</span>
					</Row>
				{/each}
			{:else}
				<Row label="History" hint="Nothing recorded yet, or the daemon is unavailable.">
					<span>Empty</span>
				</Row>
			{/if}
		</Group>
	{/if}
</ViewScaffold>

<style>
	.profile-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.provider-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
	}

	select {
		color-scheme: dark;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	button {
		white-space: nowrap;
	}
</style>
