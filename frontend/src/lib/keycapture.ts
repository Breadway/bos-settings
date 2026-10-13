// Turn a browser KeyboardEvent into the Hyprland bind (modifiers + keysym name)
// the user meant.
//
// Hyprland binds by keysym, i.e. by what a key *produces* on the active layout.
// `KeyboardEvent.code` is the physical (US-QWERTY named) position instead, so on
// AZERTY the key labelled "A" reports code `KeyQ`: using `code` for letters saved
// the wrong key (BSET-32). So printable keys are read from `event.key`, and
// `code` is only used for keys that have no printable `key` (dead keys, media
// keys the browser names generically).

export interface KeyEventLike {
	key: string;
	code: string;
	metaKey: boolean;
	ctrlKey: boolean;
	altKey: boolean;
	shiftKey: boolean;
}

export interface Captured {
	mods: string[];
	key: string;
}

const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "OS", "AltGraph", "Super", "Hyper"]);

/** Keys whose `event.key` is a name, not a character. */
const NAMED_KEYS: Record<string, string> = {
	Enter: "RETURN",
	Escape: "ESCAPE",
	Backspace: "BACKSPACE",
	Tab: "TAB",
	ArrowLeft: "left",
	ArrowRight: "right",
	ArrowUp: "up",
	ArrowDown: "down",
	Home: "Home",
	End: "End",
	PageUp: "Prior",
	PageDown: "Next",
	Insert: "Insert",
	Delete: "Delete",
	PrintScreen: "Print",
	Pause: "Pause",
	ContextMenu: "Menu",
	AudioVolumeUp: "XF86AudioRaiseVolume",
	AudioVolumeDown: "XF86AudioLowerVolume",
	AudioVolumeMute: "XF86AudioMute",
	MicrophoneVolumeMute: "XF86AudioMicMute",
	BrightnessUp: "XF86MonBrightnessUp",
	BrightnessDown: "XF86MonBrightnessDown",
	MediaTrackNext: "XF86AudioNext",
	MediaTrackPrevious: "XF86AudioPrev",
	MediaPlayPause: "XF86AudioPlay",
	MediaStop: "XF86AudioStop",
};

/** `event.code` values for keys whose `event.key` is not useful. */
const NAMED_CODES: Record<string, string> = {
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
	LaunchApp2: "XF86Calculator",
	LaunchMail: "XF86Mail",
	BrowserHome: "XF86HomePage",
	Space: "SPACE",
	Enter: "RETURN",
	NumpadEnter: "KP_Enter",
};

/** Punctuation characters to xkb keysym names (unshifted characters). */
const PUNCT: Record<string, string> = {
	"[": "bracketleft",
	"]": "bracketright",
	"/": "slash",
	"\\": "backslash",
	",": "comma",
	".": "period",
	";": "semicolon",
	"'": "apostrophe",
	"-": "minus",
	"=": "equal",
	"`": "grave",
	"<": "less",
	">": "greater",
	" ": "SPACE",
};

/**
 * With Shift held, `event.key` is the shifted character ("!" for Shift+1 on a
 * US layout). Hyprland binds the base key and lists SHIFT as a modifier, so map
 * the shifted symbol back to its base character (US pairs; other layouts that
 * produce a letter or digit under Shift are already handled as such).
 */
const SHIFTED_TO_BASE: Record<string, string> = {
	"!": "1",
	"@": "2",
	"#": "3",
	$: "4",
	"%": "5",
	"^": "6",
	"&": "7",
	"*": "8",
	"(": "9",
	")": "0",
	_: "-",
	"+": "=",
	"{": "[",
	"}": "]",
	"|": "\\",
	":": ";",
	'"': "'",
	"?": "/",
	"~": "`",
};

function printableToKeysym(ch: string, shift: boolean): string {
	let c = ch;
	if (shift && SHIFTED_TO_BASE[c]) c = SHIFTED_TO_BASE[c];
	if (/^[a-zA-Z]$/.test(c)) return c.toUpperCase();
	if (/^[0-9]$/.test(c)) return c;
	if (PUNCT[c]) return PUNCT[c];
	// Anything else (accented letters, other symbols): xkb understands Uxxxx.
	const cp = c.codePointAt(0) ?? 0;
	return `U${cp.toString(16).toUpperCase().padStart(4, "0")}`;
}

/** Physical-key fallback for dead keys and other non-printable-`key` events. */
function fromCode(code: string): string | null {
	if (NAMED_CODES[code]) return NAMED_CODES[code];
	const letter = /^Key([A-Z])$/.exec(code);
	if (letter) return letter[1];
	const digit = /^Digit([0-9])$/.exec(code);
	if (digit) return digit[1];
	if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
	const np = /^Numpad([0-9])$/.exec(code);
	if (np) return `KP_${np[1]}`;
	return null;
}

/**
 * The bind a key press stands for, or `null` for a press that is not a usable
 * key yet (a bare modifier, an unidentified key).
 */
export function hyprFromKeyEvent(e: KeyEventLike): Captured | null {
	if (MODIFIER_KEYS.has(e.key)) return null;
	const mods: string[] = [];
	if (e.metaKey) mods.push("SUPER");
	if (e.ctrlKey) mods.push("CTRL");
	if (e.altKey) mods.push("ALT");
	if (e.shiftKey) mods.push("SHIFT");

	let key: string | null = null;
	if (NAMED_KEYS[e.key]) {
		key = NAMED_KEYS[e.key];
	} else if (/^F([1-9]|1[0-9]|2[0-4])$/.test(e.key)) {
		key = e.key;
	} else if ([...e.key].length === 1) {
		key = printableToKeysym(e.key, e.shiftKey);
	} else if (e.key === "Dead" || e.key === "Unidentified" || e.key === "Process") {
		key = fromCode(e.code);
	} else {
		key = fromCode(e.code);
	}
	if (!key) return null;
	return { mods, key };
}
