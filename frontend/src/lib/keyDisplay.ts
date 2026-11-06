// Display names for Hyprland bind tokens. binds.json stores canonical xkb
// keysym names (`comma`, `RETURN`, `Delete`, ...) and modifier names
// (`SUPER`, ...); the UI shows the glyph or word a person reads on the key.
// This is presentation only: stored combos keep the exact tokens they had.

export const KEY_LABELS: Record<string, string> = {
	// Modifiers.
	SUPER: "Super",
	CTRL: "Ctrl",
	ALT: "Alt",
	SHIFT: "Shift",

	// Punctuation keysyms.
	comma: ",",
	slash: "/",
	period: ".",
	semicolon: ";",
	apostrophe: "'",
	bracketleft: "[",
	bracketright: "]",
	minus: "-",
	equal: "=",
	grave: "`",
	backslash: "\\",
	plus: "+",

	// Named keys.
	RETURN: "Enter",
	SPACE: "Space",
	ESCAPE: "Esc",
	TAB: "Tab",
	BACKSPACE: "Backspace",
	DELETE: "Delete",

	// Arrow and navigation keys. The shipped binds.json stores these
	// lower-case (`left`, `right`, `up`, `down`), unlike the upper-case
	// named keys above.
	left: "Left",
	right: "Right",
	up: "Up",
	down: "Down",

	// Mouse tokens. `mouse:272`/`mouse:273` are Linux input button codes
	// (BTN_LEFT/BTN_RIGHT) and `mouse_down`/`mouse_up` are scroll axes; the
	// shipped binds.json uses those exact spellings.
	"mouse:272": "Left click",
	"mouse:273": "Right click",
	mouse_down: "Scroll down",
	mouse_up: "Scroll up",

	// Media and function keys.
	XF86AudioRaiseVolume: "Vol +",
	XF86AudioLowerVolume: "Vol -",
	XF86AudioMute: "Mute",
	XF86AudioMicMute: "Mic mute",
	XF86MonBrightnessUp: "Bright +",
	XF86MonBrightnessDown: "Bright -",
	XF86AudioNext: "Next",
	XF86AudioPrev: "Prev",
	XF86AudioPlay: "Play",
	XF86Calculator: "Calculator",
	Print: "Print",
};

// Case-insensitive fallback, because binds.json and the key-capture code
// disagree on casing for the same keysym (`RETURN` vs `Return`).
const CASE_INSENSITIVE: Map<string, string> = new Map(
	Object.entries(KEY_LABELS).map(([token, label]) => [token.toLowerCase(), label]),
);

/** The display label for one token, or the token itself when it is not mapped. */
export function keyLabel(token: string | null | undefined): string {
	const trimmed = (token ?? "").trim();
	if (!trimmed) return "";
	return KEY_LABELS[trimmed] ?? CASE_INSENSITIVE.get(trimmed.toLowerCase()) ?? trimmed;
}

/**
 * The labels for a combination. A combo without a key shows only its
 * modifiers, and an empty combo shows nothing rather than a stray separator.
 */
export function comboLabels(
	mods: readonly string[] | null | undefined,
	key: string | null | undefined,
): string[] {
	return [...(mods ?? []), key ?? ""].map(keyLabel).filter((label) => label.length > 0);
}

/** The existing "Super + U" text convention, also used for conflict messages. */
export function comboText(
	mods: readonly string[] | null | undefined,
	key: string | null | undefined,
): string {
	return comboLabels(mods, key).join(" + ");
}
