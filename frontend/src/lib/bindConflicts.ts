// Conflict detection for keyboard shortcuts: is a combo already taken?

const MOD_ALIASES: Record<string, string> = {
	SUPER: "SUPER",
	WIN: "SUPER",
	META: "SUPER",
	MOD4: "SUPER",
	CTRL: "CTRL",
	CONTROL: "CTRL",
	ALT: "ALT",
	MOD1: "ALT",
	SHIFT: "SHIFT",
};

export function normalizeMods(mods: string[]): string[] {
	const out = new Set<string>();
	for (const m of mods) {
		const up = m.trim().toUpperCase();
		if (up) out.add(MOD_ALIASES[up] ?? up);
	}
	return [...out].sort();
}

/** Mods a bind effectively uses: its own list, or the file's default when it has none. */
export function effectiveMods(mods: string[] | undefined, defaultMods: string[]): string[] {
	return normalizeMods(mods === undefined ? defaultMods : mods);
}

/** Stable identity of a combo: sorted modifiers plus the key, case-insensitive. */
export function comboId(mods: string[], key: string): string {
	return `${normalizeMods(mods).join("+")}|${key.trim().toLowerCase()}`;
}

export interface BindCombo {
	/** `undefined` means "not set", so the file's default mods apply. */
	mods?: string[];
	key?: string;
}

/**
 * The first bind in `binds` that already uses `candidate`'s combo, ignoring
 * `skip` (the bind being edited). Binds without a key never conflict.
 */
export function findConflict<T extends BindCombo>(
	binds: readonly T[],
	candidate: { mods: string[]; key: string },
	defaultMods: string[],
	skip?: T,
): T | undefined {
	const want = comboId(candidate.mods, candidate.key);
	return binds.find((b) => {
		if (b === skip || !b.key || !b.key.trim()) return false;
		return comboId(effectiveMods(b.mods, defaultMods), b.key) === want;
	});
}
