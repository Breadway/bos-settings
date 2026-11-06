// Run with: node --experimental-strip-types frontend/src/lib/keyDisplay.test.mjs
import assert from "node:assert/strict";
import { KEY_LABELS, comboLabels, comboText, keyLabel } from "./keyDisplay.ts";

// Every token the page maps, including the existing media keys.
const expected = {
	SUPER: "Super",
	CTRL: "Ctrl",
	ALT: "Alt",
	SHIFT: "Shift",
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
	RETURN: "Enter",
	SPACE: "Space",
	ESCAPE: "Esc",
	TAB: "Tab",
	BACKSPACE: "Backspace",
	DELETE: "Delete",
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

assert.deepEqual(KEY_LABELS, expected, "the display map covers exactly the documented tokens");
for (const [token, label] of Object.entries(expected)) {
	assert.equal(keyLabel(token), label, token);
	// binds.json uses lower-case punctuation, the capture code upper-case
	// named keys; both spellings must resolve to the same label.
	assert.equal(keyLabel(token.toLowerCase()), label, `${token} lower-case`);
	assert.equal(keyLabel(token.toUpperCase()), label, `${token} upper-case`);
}

// Unknown tokens pass through unchanged instead of disappearing.
assert.equal(keyLabel("U"), "U");
assert.equal(keyLabel("h"), "h");
assert.equal(keyLabel("HyperKey"), "HyperKey");
assert.equal(keyLabel("F11"), "F11");

// Empty tokens are dropped so a combo never renders a stray separator.
assert.equal(keyLabel(""), "");
assert.equal(keyLabel("   "), "");
assert.equal(keyLabel(null), "");
assert.equal(keyLabel(undefined), "");

// Combinations keep the existing "Super + U" convention.
assert.deepEqual(comboLabels(["SUPER"], "U"), ["Super", "U"]);
assert.equal(comboText(["SUPER"], "comma"), "Super + ,");
assert.equal(comboText(["SUPER", "SHIFT"], "slash"), "Super + Shift + /");
assert.equal(comboText(["SUPER"], "RETURN"), "Super + Enter");

// A combo with no key shows only its modifiers, and an empty combo is empty.
assert.deepEqual(comboLabels(["SUPER", "CTRL"], ""), ["Super", "Ctrl"]);
assert.equal(comboText(["SUPER"], ""), "Super");
assert.deepEqual(comboLabels([], ""), []);
assert.equal(comboText([], ""), "");
assert.deepEqual(comboLabels(null, null), []);

// A key that is itself "+" stays a distinct token, with the existing
// space-separated plus convention rather than running into the separator.
assert.deepEqual(comboLabels(["SUPER"], "plus"), ["Super", "+"]);
assert.equal(comboText(["SUPER"], "plus"), "Super + +");
assert.deepEqual(comboLabels(["SUPER"], "+"), ["Super", "+"]);
assert.equal(comboText(["SUPER"], "+"), "Super + +");

console.log("keyDisplay: all assertions passed");
