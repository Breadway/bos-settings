<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import TextField from "$lib/components/TextField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";
	import Switch from "$lib/components/Switch.svelte";

	// Mirrors src-tauri/src/commands/keybinds.rs's `Bind`  -  `action`/`key`/
	// `mods` are the only fields every bind has; everything else (`command`,
	// `direction`, `workspace`, `options`, breadhelp's `label`/`category`/
	// `demo_cmd`, ...) round-trips through serde's `#[serde(flatten)]` as
	// arbitrary sibling keys on the same JSON object.
	interface Bind {
		action: string;
		key?: string;
		mods?: string[];
		[extra: string]: unknown;
	}
	interface BindsFile {
		active_layout: string;
		default_mods: string[];
		globals: Bind[];
		common: Bind[];
		layouts: Record<string, Bind[]>;
		bindings: Bind[];
	}
	type SchemaKind = "flat" | "multi_layout" | "unknown";
	interface BindsPayload {
		kind: SchemaKind;
		file: BindsFile;
	}

	// Every action actually seen in real binds.json files (BOS's shipped
	// flat schema and this app's own dev MultiLayout config)  -  an `action`
	// dropdown beats free-typing a dispatcher name from memory. "Custom…"
	// keeps anything not in this list reachable without blocking on it.
	const KNOWN_ACTIONS = [
		"exec",
		"close",
		"fullscreen",
		"float",
		"pseudo",
		"resize",
		"focus",
		"focus_last",
		"move",
		"move_dir",
		"resize_dir",
		"drag",
		"layout",
		"exit",
	] as const;

	// The editable, per-row UI state a bind gets flattened into on load and
	// reconstructed back into a `Bind` on save  -  same trade-off the GTK
	// version made with individual `Entry` widgets per field.
	interface EditRow {
		action: string;
		key: string;
		mods: string;
		// Whether `mods` has been explicitly set (present in the loaded JSON,
		// or touched by the user since). `None`/absent means "fall back to
		// default_mods"; `Some([])`  -  an explicitly *empty* mods list  -  means
		// "use no modifiers at all, even though default_mods exists" (real
		// BOS binds rely on this for e.g. bare media keys). Collapsing both
		// cases to "omit when empty" would silently turn an explicit
		// no-mods override back into "use the default" on next save.
		modsTouched: boolean;
		// The single source of truth for everything beyond action/key/mods
		// (command, direction, workspace, x/y, layout, options, breadhelp's
		// label/category/demo_cmd, ...). Dedicated per-action fields below
		// read/write specific keys of this object directly, so any sibling
		// key they don't know about (label/category/demo_cmd, or an action
		// shape this editor has no dedicated fields for) survives untouched.
		extraValue: Record<string, unknown>;
		// Raw-JSON view of `extraValue`, kept in sync both directions  - 
		// only actually shown when `advancedOpen` is true, or for an action
		// not in `KNOWN_ACTIONS` (nothing dedicated to show instead).
		extraText: string;
		extraError: boolean;
		advancedOpen: boolean;
		capturing: boolean;
	}

	const ACTION_LABELS: Record<string, string> = {
		exec: "Run command",
		close: "Close window",
		fullscreen: "Fullscreen",
		float: "Float window",
		pseudo: "Fake fullscreen",
		resize: "Resize",
		focus: "Go to workspace",
		focus_last: "Last workspace",
		move: "Move window to workspace",
		move_dir: "Move window",
		resize_dir: "Resize window",
		layout: "Layout",
		drag: "Mouse drag",
		exit: "Exit Hyprland",
	};

	const KEY_LABELS: Record<string, string> = {
		RETURN: "Enter",
		SPACE: "Space",
		ESCAPE: "Esc",
		BACKSPACE: "Backspace",
		TAB: "Tab",
		SUPER: "Super",
		CTRL: "Ctrl",
		ALT: "Alt",
		SHIFT: "Shift",
		XF86AudioRaiseVolume: "Vol +",
		XF86AudioLowerVolume: "Vol -",
		XF86AudioMute: "Mute",
		XF86AudioMicMute: "Mic mute",
		XF86MonBrightnessUp: "Bright +",
		XF86MonBrightnessDown: "Bright -",
		XF86AudioNext: "Next",
		XF86AudioPrev: "Prev",
		XF86AudioPlay: "Play",
		Print: "Print",
	};

	const CODE_TO_HYPR: Record<string, string> = {
		Space: "SPACE",
		Enter: "RETURN",
		Escape: "ESCAPE",
		Backspace: "BACKSPACE",
		Tab: "TAB",
		AudioVolumeUp: "XF86AudioRaiseVolume",
		AudioVolumeDown: "XF86AudioLowerVolume",
		AudioVolumeMute: "XF86AudioMute",
		AudioMicMute: "XF86AudioMicMute",
		BrightnessUp: "XF86MonBrightnessUp",
		BrightnessDown: "XF86MonBrightnessDown",
		MediaTrackNext: "XF86AudioNext",
		MediaTrackPrevious: "XF86AudioPrev",
		MediaPlayPause: "XF86AudioPlay",
		PrintScreen: "Print",
	};

	function keyLabel(k: string): string {
		return KEY_LABELS[k] ?? k;
	}

	function bindTitle(row: EditRow): string {
		if (row.action === "exec") {
			const cmd = String(row.extraValue.command ?? "");
			if (cmd.includes("set-volume") && cmd.includes("+")) return "Volume up";
			if (cmd.includes("set-volume") && cmd.includes("-")) return "Volume down";
			if (cmd.includes("set-mute") && cmd.includes("SINK")) return "Mute";
			if (cmd.includes("set-mute") && cmd.includes("SOURCE")) return "Mute mic";
			if (cmd.includes("brightnessctl") && cmd.includes("+")) return "Brightness up";
			if (cmd.includes("brightnessctl") && cmd.includes("-")) return "Brightness down";
			if (cmd.includes("playerctl next")) return "Next track";
			if (cmd.includes("playerctl previous")) return "Previous track";
			if (cmd.includes("playerctl play-pause")) return "Play/pause";
			if (cmd) return cmd.split(/\s+/)[0].split("/").pop() ?? "Command";
		}
		return ACTION_LABELS[row.action] ?? row.action;
	}

	function keyChips(row: EditRow): string[] {
		const mods = textToMods(row.mods);
		const key = row.key.trim();
		return [...mods, key].filter(Boolean).map(keyLabel);
	}

	function hyprFromEvent(e: KeyboardEvent): { mods: string[]; key: string } | null {
		if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return null;
		const mods: string[] = [];
		if (e.metaKey) mods.push("SUPER");
		if (e.ctrlKey) mods.push("CTRL");
		if (e.altKey) mods.push("ALT");
		if (e.shiftKey) mods.push("SHIFT");
		let key = CODE_TO_HYPR[e.code] ?? CODE_TO_HYPR[e.key];
		if (!key) {
			if (e.code.startsWith("Key") && e.code.length === 4) key = e.code.slice(3);
			else if (e.code.startsWith("Digit")) key = e.code.slice(5);
			else if (e.code.startsWith("F") && /^F\d+$/.test(e.code)) key = e.code;
			else key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
		}
		return { mods, key };
	}

	function allRows(): EditRow[] {
		return [...globalsRows, ...commonRows, ...bindingsRows, ...Object.values(layoutRows).flat()];
	}

	function beginCapture(row: EditRow) {
		for (const r of allRows()) r.capturing = false;
		row.capturing = true;
	}

	function syncExtraText(row: EditRow) {
		row.extraText = Object.keys(row.extraValue).length ? JSON.stringify(row.extraValue) : "";
		row.extraError = false;
	}

	/** Mutates one top-level key of `row.extraValue` (deleting it when `value` is empty/undefined) and re-syncs the raw-JSON view. */
	function setExtra(row: EditRow, key: string, value: unknown) {
		if (value === undefined || value === "") {
			delete row.extraValue[key];
		} else {
			row.extraValue[key] = value;
		}
		syncExtraText(row);
	}

	function getOption(row: EditRow, key: "locked" | "repeating"): boolean {
		const options = row.extraValue.options as Record<string, unknown> | undefined;
		return Boolean(options?.[key]);
	}
	function setOption(row: EditRow, key: "locked" | "repeating", value: boolean) {
		const options = { ...((row.extraValue.options as Record<string, unknown>) ?? {}) };
		if (value) options[key] = true;
		else delete options[key];
		if (Object.keys(options).length) row.extraValue.options = options;
		else delete row.extraValue.options;
		syncExtraText(row);
	}

	// Hyprland workspace refs mix bare integers ("1") and relative tokens
	// ("e+1", "e-1") in the same field  -  keep whichever shape the user typed
	// instead of forcing everything through one type.
	function parseWorkspaceValue(s: string): string | number | undefined {
		const trimmed = s.trim();
		if (!trimmed) return undefined;
		return /^-?\d+$/.test(trimmed) ? Number(trimmed) : trimmed;
	}

	function modsToText(mods?: string[]): string {
		return (mods ?? []).join(", ");
	}
	function textToMods(s: string): string[] {
		return s
			.split(",")
			.map((s) => s.trim())
			.filter((s) => s.length > 0);
	}

	function bindToRow(b: Bind): EditRow {
		const { action, key, mods, ...extra } = b;
		const row: EditRow = {
			action: action ?? "",
			key: (key as string | undefined) ?? "",
			mods: modsToText(mods),
			modsTouched: mods !== undefined,
			extraValue: extra,
			extraText: "",
			extraError: false,
			advancedOpen: false,
			capturing: false,
		};
		syncExtraText(row);
		return row;
	}

	function rowToBind(r: EditRow): Bind {
		// `drag` binds are only meaningful as a mouse bind  -  there's no
		// dedicated field for `options.mouse` (nothing to configure, it's
		// always true), so pin it here rather than exposing a checkbox
		// whose only correct state is "on".
		const extra = r.action === "drag" ? { ...r.extraValue, options: { ...(r.extraValue.options as object), mouse: true } } : r.extraValue;
		return {
			action: r.action,
			...(r.key.trim() ? { key: r.key.trim() } : {}),
			...(r.modsTouched ? { mods: textToMods(r.mods) } : {}),
			...extra,
		};
	}

	function onExtraInput(row: EditRow, text: string) {
		row.extraText = text;
		const trimmed = text.trim();
		if (!trimmed) {
			row.extraValue = {};
			row.extraError = false;
			return;
		}
		try {
			const parsed = JSON.parse(trimmed);
			if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
				row.extraValue = parsed;
				row.extraError = false;
			} else {
				row.extraError = true;
			}
		} catch {
			row.extraError = true;
		}
	}

	function emptyRow(): EditRow {
		return {
			action: "exec",
			key: "",
			mods: "",
			modsTouched: false,
			extraValue: {},
			extraText: "",
			extraError: false,
			advancedOpen: false,
			capturing: false,
		};
	}

	let loaded = $state(false);
	let kind = $state<SchemaKind>("unknown");
	let activeLayout = $state("");
	let defaultMods = $state("");
	let globalsRows = $state<EditRow[]>([]);
	let commonRows = $state<EditRow[]>([]);
	let bindingsRows = $state<EditRow[]>([]);
	let layoutRows = $state<Record<string, EditRow[]>>({});
	let layoutOrder = $state<string[]>([]);
	let newLayoutName = $state("");
	let loadError = $state("");

	onMount(() => {
		const onKey = (e: KeyboardEvent) => {
			const row = allRows().find((r) => r.capturing);
			if (!row) return;
			const mapped = hyprFromEvent(e);
			if (!mapped) return;
			e.preventDefault();
			row.mods = mapped.mods.join(", ");
			row.modsTouched = true;
			row.key = mapped.key;
			row.capturing = false;
		};
		window.addEventListener("keydown", onKey);
		void loadKeybinds();
		// Register cleanup synchronously so Svelte types the onMount
		// callback as returning a function (not a Promise-of-function).
		return () => window.removeEventListener("keydown", onKey);
	});

	async function loadKeybinds() {
		try {
			const p = await invoke<BindsPayload>("get_keybinds");
			kind = p.kind;
			// Rust's `#[serde(skip_serializing_if = ...)]` on every one of
			// these fields means an empty one is OMITTED from the JSON
			// entirely, not sent as `[]`/`{}`/`""`  -  every field here needs
			// a `??` fallback, not just the ones that are "usually" empty.
			activeLayout = p.file.active_layout ?? "";
			defaultMods = modsToText(p.file.default_mods ?? []);
			globalsRows = (p.file.globals ?? []).map(bindToRow);
			commonRows = (p.file.common ?? []).map(bindToRow);
			bindingsRows = (p.file.bindings ?? []).map(bindToRow);
			const layouts: Record<string, EditRow[]> = {};
			for (const [name, binds] of Object.entries(p.file.layouts ?? {})) layouts[name] = binds.map(bindToRow);
			layoutRows = layouts;
			layoutOrder = Object.keys(layouts);
			loaded = true;
		} catch (e) {
			loadError = String(e);
		}
	}

	function addLayout() {
		const name = newLayoutName.trim();
		if (!name || layoutOrder.includes(name)) return;
		layoutRows[name] = [];
		layoutOrder = [...layoutOrder, name];
		newLayoutName = "";
	}
	function removeLayout(name: string) {
		if (!confirm(`Remove layout "${name}"? Deletes every bind defined under this layout. This can't be undone here.`)) return;
		delete layoutRows[name];
		layoutOrder = layoutOrder.filter((n) => n !== name);
		if (activeLayout === name) activeLayout = layoutOrder[0] ?? "";
	}

	// A row whose "extra" column currently holds text that doesn't parse as
	// JSON keeps its *last successfully parsed* value in `extraValue` (see
	// `onExtraInput`) rather than losing it on every keystroke while the
	// user is mid-edit  -  but that means saving while such a row is still
	// showing invalid/incomplete text would silently write that stale (or,
	// for a brand new row, empty) value instead of what's actually on
	// screen. Block save entirely until every row's extra JSON is valid, so
	// an in-progress edit can never be the thing that quietly drops a
	// bind's `command`/`direction`/`workspace`/etc.
	function hasInvalidExtraJson(): boolean {
		const all = [...globalsRows, ...commonRows, ...bindingsRows, ...Object.values(layoutRows).flat()];
		return all.some((r) => r.extraError);
	}

	async function save() {
		if (hasInvalidExtraJson()) {
			throw new Error("Fix the invalid JSON in the highlighted field(s) before saving.");
		}
		const file: BindsFile = {
			active_layout: activeLayout,
			default_mods: textToMods(defaultMods),
			globals: globalsRows.map(rowToBind),
			common: commonRows.map(rowToBind),
			layouts: Object.fromEntries(layoutOrder.map((name) => [name, (layoutRows[name] ?? []).map(rowToBind)])),
			bindings: bindingsRows.map(rowToBind),
		};
		await invoke("save_keybinds", { file, kind });
	}
</script>

{#snippet extraFields(row: EditRow)}
	{#if row.action === "exec"}
		<input
			class="extra-wide"
			type="text"
			placeholder="Command to run"
			value={(row.extraValue.command as string) ?? ""}
			oninput={(e) => setExtra(row, "command", e.currentTarget.value)}
		/>
		<label class="mini-switch">
			<Switch
				ariaLabel="Repeat while held"
				bind:value={() => getOption(row, "repeating"), (v) => setOption(row, "repeating", v)}
			/>
			Repeat while held
		</label>
		<label class="mini-switch">
			<Switch
				ariaLabel="Ignore keyboard lock"
				bind:value={() => getOption(row, "locked"), (v) => setOption(row, "locked", v)}
			/>
			Works when locked
		</label>
	{:else if row.action === "focus" || row.action === "move"}
		<input
			class="extra-narrow"
			type="text"
			placeholder="Workspace (e.g. 3, e+1)"
			value={String(row.extraValue.workspace ?? "")}
			oninput={(e) => setExtra(row, "workspace", parseWorkspaceValue(e.currentTarget.value))}
		/>
	{:else if row.action === "move_dir"}
		<select
			class="extra-narrow"
			value={(row.extraValue.direction as string) ?? ""}
			onchange={(e) => setExtra(row, "direction", e.currentTarget.value || undefined)}
		>
			<option value="" disabled>Direction…</option>
			<option value="left">Left</option>
			<option value="right">Right</option>
			<option value="up">Up</option>
			<option value="down">Down</option>
		</select>
	{:else if row.action === "resize_dir"}
		<input
			class="extra-tiny"
			type="number"
			placeholder="x"
			value={row.extraValue.x !== undefined ? String(row.extraValue.x) : ""}
			oninput={(e) => setExtra(row, "x", e.currentTarget.value === "" ? undefined : Number(e.currentTarget.value))}
		/>
		<input
			class="extra-tiny"
			type="number"
			placeholder="y"
			value={row.extraValue.y !== undefined ? String(row.extraValue.y) : ""}
			oninput={(e) => setExtra(row, "y", e.currentTarget.value === "" ? undefined : Number(e.currentTarget.value))}
		/>
		<label class="mini-switch">
			<Switch
				ariaLabel="Repeat while held"
				bind:value={() => getOption(row, "repeating"), (v) => setOption(row, "repeating", v)}
			/>
			Repeat while held
		</label>
	{:else if row.action === "layout"}
		<input
			class="extra-narrow"
			type="text"
			placeholder="Layout command (e.g. togglesplit)"
			value={(row.extraValue.layout as string) ?? ""}
			oninput={(e) => setExtra(row, "layout", e.currentTarget.value)}
		/>
	{:else if row.action === "drag"}
		<span class="extra-note">Mouse-drag bind.</span>
	{:else if row.action === "close" || row.action === "fullscreen" || row.action === "float" || row.action === "pseudo" || row.action === "resize" || row.action === "focus_last" || row.action === "exit"}
		<span class="extra-note">No extra options for this action.</span>
	{:else}
		<input
			class="extra-wide"
			class:error={row.extraError}
			type="text"
			placeholder={'{"command": "..."}'}
			value={row.extraText}
			oninput={(e) => onExtraInput(row, e.currentTarget.value)}
		/>
	{/if}

	{#if KNOWN_ACTIONS.includes(row.action as (typeof KNOWN_ACTIONS)[number]) && row.action !== "drag"}
		<button type="button" class="advanced-toggle" onclick={() => (row.advancedOpen = !row.advancedOpen)}>
			{row.advancedOpen ? "Hide raw JSON" : "Advanced"}
		</button>
	{/if}
{/snippet}

{#snippet section(rows: EditRow[], onAdd: () => void, onRemove: (i: number) => void)}
	<div class="rows">
		{#each rows as row, i (i)}
			<div class="bind-card">
				<div class="bind-top">
					<span class="what">{bindTitle(row)}</span>
					<button type="button" class="capture" class:listening={row.capturing} onclick={() => beginCapture(row)}>
						{#if row.capturing}
							Press a key
						{:else if keyChips(row).length}
							{#each keyChips(row) as k, ki (`${k}-${ki}`)}
								{#if ki > 0}<span class="plus">+</span>{/if}
								<span class="kbd">{k}</span>
							{/each}
						{:else}
							Set shortcut
						{/if}
					</button>
					<button type="button" class="remove" onclick={() => onRemove(i)}>Remove</button>
				</div>
				<div class="bind-bottom">
					{@render extraFields(row)}
				</div>
				{#if row.advancedOpen && KNOWN_ACTIONS.includes(row.action as (typeof KNOWN_ACTIONS)[number]) && row.action !== "drag"}
					<input
						class="extra-wide"
						class:error={row.extraError}
						type="text"
						placeholder={'{"command": "..."}'}
						value={row.extraText}
						oninput={(e) => onExtraInput(row, e.currentTarget.value)}
					/>
				{/if}
			</div>
		{/each}
	</div>
	<button type="button" class="add" onclick={onAdd}>Add bind</button>
{/snippet}

<ViewScaffold title="Keybinds">
	{#if loadError}
		<Group title="Failed to load" wide>
			<Hint text={loadError} />
		</Group>
	{/if}
	{#if loaded}
		{#if kind === "unknown"}
			<Group title="Keybinds" wide>
				<Hint
					text={`binds.json's schema wasn't recognized (expected a "bindings" key, or one of "globals"/"common"/"layouts"). Nothing below is editable, and Save is disabled, so the file on disk isn't at risk of being silently overwritten with the wrong shape. Fix or remove the file by hand, then reopen this panel.`}
				/>
			</Group>
		{:else if kind === "flat"}
			<Group
				title="Defaults"
				hint="Click a shortcut, then press the keys. Save when you are done."
			>
				<TextField label="Default mods" bind:value={defaultMods} placeholder="SUPER" />
			</Group>

			<Group title="Bindings" wide>
				{@render section(
					bindingsRows,
					() => bindingsRows.push(emptyRow()),
					(i) => bindingsRows.splice(i, 1)
				)}
			</Group>

			<SaveButton onSave={save} />
		{:else}
			<Group
				title="Layout"
				hint="Click a shortcut, then press the keys. Save when you are done."
			>
				{#if layoutOrder.length > 0}
					<SelectField label="Active layout" bind:value={activeLayout} options={layoutOrder} />
				{/if}
				<TextField label="Default mods" bind:value={defaultMods} placeholder="SUPER" />
			</Group>

			<Group title="Media & function keys (globals)" wide>
				{@render section(
					globalsRows,
					() => globalsRows.push(emptyRow()),
					(i) => globalsRows.splice(i, 1)
				)}
			</Group>

			<Group title="Common (every layout)" wide>
				{@render section(
					commonRows,
					() => commonRows.push(emptyRow()),
					(i) => commonRows.splice(i, 1)
				)}
			</Group>

			{#each layoutOrder as name (name)}
				<Group title={`Layout: ${name}`} wide>
					<button type="button" class="remove-layout" onclick={() => removeLayout(name)}>Remove layout</button>
					{@render section(
						layoutRows[name] ?? [],
						() => (layoutRows[name] ?? (layoutRows[name] = [])).push(emptyRow()),
						(i) => layoutRows[name]?.splice(i, 1)
					)}
				</Group>
			{/each}

			<Group title="Add layout">
				<div class="add-layout-row">
					<input type="text" bind:value={newLayoutName} placeholder="new layout name" />
					<button type="button" onclick={addLayout}>Add layout</button>
				</div>
			</Group>

			<SaveButton onSave={save} />
		{/if}
	{/if}
</ViewScaffold>

<style>
	.rows {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-bottom: var(--space-sm, 8px);
	}

	.bind-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-xs, 4px);
		padding: 10px 2px;
		border-top: 1px solid var(--line);
	}

	.bind-card:first-child {
		border-top: none;
		padding-top: 0;
	}

	.bind-top,
	.bind-bottom {
		display: flex;
		align-items: center;
		gap: var(--space-sm, 8px);
		flex-wrap: wrap;
	}

	.what {
		flex: 1;
		min-width: 12ch;
	}

	.capture {
		display: flex;
		align-items: center;
		gap: 4px;
		background: color-mix(in srgb, var(--fg) 6%, transparent);
		border: 1px solid var(--line);
		border-radius: 10px;
		padding: 6px 10px;
		color: inherit;
		cursor: pointer;
		min-height: 32px;
	}

	.capture.listening {
		border-color: var(--accent);
		color: var(--accent);
	}

	.plus {
		opacity: 0.4;
		padding: 0 1px;
	}

	.bind-card input,
	.bind-card select {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.bind-card input:focus,
	.bind-card select:focus {
		outline: none;
		border-color: var(--accent);
	}

	.bind-card input.error {
		border-color: var(--red);
	}

	.extra-wide {
		flex: 1;
		min-width: 16ch;
		font-family: monospace;
		font-size: var(--font-size-secondary, 12px);
	}

	.extra-narrow {
		width: 20ch;
	}

	.extra-tiny {
		width: 7ch;
	}

	.extra-note {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
	}

	.mini-switch {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: var(--font-size-secondary, 12px);
		opacity: 0.85;
	}

	.advanced-toggle {
		background: transparent;
		border: none;
		color: var(--on-surface);
		opacity: 0.6;
		cursor: pointer;
		font-size: var(--font-size-secondary, 12px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
		margin-left: auto;
	}

	.advanced-toggle:hover {
		opacity: 1;
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
		flex-shrink: 0;
	}

	.add {
		background-color: var(--surface);
		color: var(--on-surface);
		align-self: flex-start;
	}

	.remove-layout {
		background-color: var(--red);
		color: var(--on-red);
		margin-bottom: var(--space-sm, 8px);
		align-self: flex-start;
	}

	.add-layout-row {
		display: flex;
		gap: var(--space-sm, 8px);
	}

	.add-layout-row input {
		background-color: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	.add-layout-row button {
		background-color: var(--accent);
		color: var(--on-accent);
	}
</style>
