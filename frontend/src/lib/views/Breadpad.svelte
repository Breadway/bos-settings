<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import FileField from "$lib/components/FileField.svelte";
	import PasswordField from "$lib/components/PasswordField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import NumberField from "$lib/components/NumberField.svelte";
	import TagsField from "$lib/components/TagsField.svelte";
	import Row from "$lib/components/Row.svelte";
	import { noteShortcutText, type BindsFileLike } from "$lib/noteBind";

	interface BindsPayloadLike {
		file?: BindsFileLike;
	}

	interface BreadpadConfig {
		default_type: string;
		workspace_tag: boolean;
		snooze_options: string[];
		archive_after_days: number;
		model_path: string;
		tokenizer_path: string;
		ollama_enabled: boolean;
		ollama_endpoint: string;
		ollama_model: string;
		ollama_confidence_threshold: number;
		reminders_default_morning: string;
		reminders_missed_grace_minutes: number;
		calendar_enabled: boolean;
		calendar_url: string;
		calendar_username: string;
		calendar_password: string;
	}

	let cfg = $state<BreadpadConfig | null>(null);

	// The confidence threshold is an f64 (0-1, step 0.05); binding a plain
	// number input directly to it shows raw float noise (e.g. "0.6000000")
	// once the value has been nudged by the spinner. Instead we mirror it
	// into a formatted display string, and only parse it back into `cfg` as
	// a value rounded to 2 decimals.
	let confidenceText = $state("0.6");
	let confidenceLoaded = false;

	function formatConfidence(n: number): string {
		return (Math.round(n * 100) / 100).toString();
	}

	function onConfidenceInput(e: Event) {
		confidenceText = (e.currentTarget as HTMLInputElement).value;
		const parsed = parseFloat(confidenceText);
		if (!Number.isNaN(parsed) && cfg) {
			cfg.ollama_confidence_threshold = Math.round(parsed * 100) / 100;
		}
	}

	function onConfidenceBlur() {
		if (cfg) confidenceText = formatConfidence(cfg.ollama_confidence_threshold);
	}

	$effect(() => {
		if (cfg && !confidenceLoaded) {
			confidenceText = formatConfidence(cfg.ollama_confidence_threshold);
			confidenceLoaded = true;
		}
	});

	// Read the shortcut from binds.json so the copy follows the real bind.
	// A missing or unreadable keybind file only drops the key name from the
	// sentence; it must never stop the configuration from loading.
	let noteShortcut = $state<string | null>(null);
	let quickNotesHint = $derived(
		noteShortcut
			? `${noteShortcut} opens a note. BOS sorts it into a note, reminder or task.`
			: "Open a note from the keyboard. BOS sorts it into a note, reminder or task.",
	);

	onMount(async () => {
		const [cfgResult, bindsResult] = await Promise.allSettled([
			invoke<BreadpadConfig>("get_breadpad_config"),
			invoke<BindsPayloadLike>("get_keybinds"),
		]);
		if (cfgResult.status === "fulfilled") cfg = cfgResult.value;
		if (bindsResult.status === "fulfilled") noteShortcut = noteShortcutText(bindsResult.value?.file);
	});

	async function save() {
		await invoke("save_breadpad_config", { cfg });
	}
	useDraft(() => ({ cfg, confidenceText }), save, () => cfg !== null);
</script>

<ViewScaffold title="Notes">
	{#if cfg}
		<Group title="Quick notes" hint={quickNotesHint}>
			<Row label="New notes start as">
				<div class="ui-seg">{#each [["note", "Note"], ["reminder", "Reminder"], ["task", "Task"]] as [v, l] (v)}<button class:on={cfg.default_type === v} onclick={() => (cfg!.default_type = v)}>{l}</button>{/each}</div>
			</Row>
			<SwitchField label="Remember the workspace" hint="Tags each note with the workspace you wrote it on." bind:value={cfg.workspace_tag} />
			<Row label="Archive finished notes after">
				<select class="ui-input" bind:value={cfg.archive_after_days}>
					{#each [...new Set([0, 7, 14, 30, 90, cfg.archive_after_days])].sort((a, b) => a - b) as d (d)}<option value={d}>{d === 0 ? "Never" : `${d} days`}</option>{/each}
				</select>
			</Row>
			<Row label="Morning reminders at" hint="For reminders like “tomorrow morning”.">
				<input class="ui-input" style="width:90px" bind:value={cfg.reminders_default_morning} placeholder="08:00" />
			</Row>
			<TagsField label="Snooze choices" bind:value={cfg.snooze_options} />
			<details class="ui-disclosure">
				<summary>Missed reminders and smart sorting</summary>
				<NumberField label="Still show a missed reminder for (minutes)" bind:value={cfg.reminders_missed_grace_minutes} min={0} max={1440} step={5} />
				<FileField label="Sorting model" bind:value={cfg.model_path} placeholder="~/.local/share/breadpad/model/classifier.onnx" extensions={["onnx"]} />
				<FileField label="Model tokenizer" bind:value={cfg.tokenizer_path} placeholder="~/.local/share/breadpad/model/tokenizer.json" extensions={["json"]} />
				<SwitchField label="Ask a local AI model (Ollama)" hint="Runs on this computer; nothing leaves it." bind:value={cfg.ollama_enabled} />
				{#if cfg.ollama_enabled}
					<TextField label="Ollama address" bind:value={cfg.ollama_endpoint} placeholder="http://localhost:11434" />
					<TextField label="Ollama model" bind:value={cfg.ollama_model} placeholder="e.g. fastflowlm" />
					<Row label="Only trust it above" hint="How sure the model must be, from 0 to 1.">
						<input type="number" min="0" max="1" step="0.05" value={confidenceText} oninput={onConfidenceInput} onblur={onConfidenceBlur} />
					</Row>
				{/if}
			</details>
		</Group>

		<Group title="Calendar">
			<SwitchField label="Add reminders to my calendar" hint="Any CalDAV calendar, like Nextcloud." bind:value={cfg.calendar_enabled} />
			{#if cfg.calendar_enabled}
				<TextField label="Calendar address" bind:value={cfg.calendar_url} placeholder="https://host/remote.php/dav/calendars/..." />
				<TextField label="Username" bind:value={cfg.calendar_username} />
				<PasswordField label="Password" bind:value={cfg.calendar_password} />
			{/if}
		</Group>
	{/if}
</ViewScaffold>

<style>
	input[type="number"] {
		width: 10ch;
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input[type="number"]:focus {
		outline: none;
		border-color: var(--accent);
	}
</style>
