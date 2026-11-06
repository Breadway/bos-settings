// Resolve the "capture a note" shortcut from binds.json instead of naming a
// key in the Quick notes copy. The shipped BOS bind runs `breadpad`; users can
// move it, so the copy has to follow the file. When there is no usable bind
// the caller shows copy that does not name a key at all.

export interface BindLike {
	action?: string;
	key?: string;
	mods?: string[];
	command?: string;
	[extra: string]: unknown;
}

export interface BindsFileLike {
	default_mods?: string[];
	bindings?: BindLike[];
	globals?: BindLike[];
	common?: BindLike[];
	layouts?: Record<string, BindLike[]>;
}

const MODIFIER_LABELS: Record<string, string> = {
	SUPER: "Super",
	CTRL: "Ctrl",
	ALT: "Alt",
	SHIFT: "Shift",
};

/** The program a bind's command actually launches, unwrapping a shell. */
function programOf(command: string): string {
	const inner = command
		.trim()
		.replace(/^(?:\S*\/)?(?:ba)?sh\s+-c\s+/, "")
		.replace(/^['"]|['"]$/g, "");
	return (inner.split(/\s+/)[0] ?? "").split("/").pop() ?? "";
}

function isNoteBind(bind: BindLike): boolean {
	return bind.action === "exec" && typeof bind.command === "string" && programOf(bind.command) === "breadpad";
}

/** Every bind the file defines, in the order the app can edit them. */
function allBinds(file: BindsFileLike): BindLike[] {
	return [
		...(file.bindings ?? []),
		...(file.globals ?? []),
		...(file.common ?? []),
		...Object.values(file.layouts ?? {}).flat(),
	];
}

/**
 * The bind that opens breadpad's capture box, or `null` when the file has no
 * breadpad bind, has one without a key, or is not readable as an object.
 */
export function findNoteBind(file: BindsFileLike | null | undefined): BindLike | null {
	if (!file || typeof file !== "object") return null;
	const candidates = allBinds(file).filter(isNoteBind);
	const preferred = candidates.find((bind) => /capture|note/i.test(String(bind.command ?? "")));
	return preferred ?? candidates[0] ?? null;
}

/**
 * "Super + U" for a bind, or `null` when the bind has no key. Modifier names
 * are spelled out; the key token is shown as the file spells it (letters are
 * already the glyph a user reads).
 */
export function noteShortcutLabel(bind: BindLike | null, defaultMods: string[] = []): string | null {
	if (!bind) return null;
	const key = (bind.key ?? "").trim();
	if (!key) return null;
	const mods = bind.mods ?? defaultMods;
	return [...mods.map((mod) => MODIFIER_LABELS[mod] ?? mod), key].filter(Boolean).join(" + ");
}

/** The full shortcut from a binds.json payload, or `null` for the safe copy. */
export function noteShortcutText(file: BindsFileLike | null | undefined): string | null {
	const bind = findNoteBind(file);
	return noteShortcutLabel(bind, file?.default_mods ?? []);
}
