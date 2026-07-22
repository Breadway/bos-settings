<script lang="ts">
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
	import SaveButton from "$lib/components/SaveButton.svelte";

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

	onMount(async () => {
		cfg = await invoke<BreadpadConfig>("get_breadpad_config");
	});

	async function save() {
		await invoke("save_breadpad_config", { cfg });
	}
</script>

<ViewScaffold title="Notes">
	{#if cfg}
		<Group title="Capture">
			<SelectField label="Default note type" bind:value={cfg.default_type} options={["note", "reminder", "task"]} />
			<SwitchField label="Tag with active workspace" bind:value={cfg.workspace_tag} />
			<TagsField label="Snooze options" bind:value={cfg.snooze_options} />
			<NumberField label="Archive after (days)" bind:value={cfg.archive_after_days} min={0} max={3650} />
		</Group>

		<Group title="Classifier model" hint="The local model breadpad uses to guess note vs. reminder vs. task — no network needed.">
			<FileField label="ONNX model" bind:value={cfg.model_path} placeholder="~/.local/share/breadpad/model/classifier.onnx" extensions={["onnx"]} />
			<FileField label="Tokenizer" bind:value={cfg.tokenizer_path} placeholder="~/.local/share/breadpad/model/tokenizer.json" extensions={["json"]} />
		</Group>

		<Group title="Ollama (LLM classifier)" hint="Optional: ask a locally-running LLM (via Ollama) to classify notes instead of, or alongside, the ONNX model above.">
			<SwitchField label="Use Ollama" bind:value={cfg.ollama_enabled} />
			<TextField label="Endpoint" bind:value={cfg.ollama_endpoint} placeholder="http://localhost:11434" />
			<TextField label="Model" bind:value={cfg.ollama_model} placeholder="e.g. fastflowlm" />
			<Row label="Confidence threshold">
				<input
					type="number"
					min="0"
					max="1"
					step="0.05"
					value={confidenceText}
					oninput={onConfidenceInput}
					onblur={onConfidenceBlur}
				/>
			</Row>
		</Group>

		<Group title="Reminders">
			<TextField label="Default morning time" bind:value={cfg.reminders_default_morning} placeholder="7:00" />
			<NumberField label="Missed grace (minutes)" bind:value={cfg.reminders_missed_grace_minutes} min={0} max={1440} step={5} />
		</Group>

		<Group title="Calendar (CalDAV)">
			<SwitchField label="Sync to calendar" bind:value={cfg.calendar_enabled} />
			<TextField label="CalDAV URL" bind:value={cfg.calendar_url} placeholder="https://host/remote.php/dav/calendars/..." />
			<TextField label="Username" bind:value={cfg.calendar_username} />
			<PasswordField label="Password" bind:value={cfg.calendar_password} />
		</Group>

		<SaveButton onSave={save} />
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
