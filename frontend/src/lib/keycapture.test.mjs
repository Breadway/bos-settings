// Run with: node --experimental-strip-types frontend/src/lib/keycapture.test.mjs
import assert from "node:assert/strict";
import { hyprFromKeyEvent } from "./keycapture.ts";
import { comboId, effectiveMods, findConflict, normalizeMods } from "./bindConflicts.ts";

const ev = (key, code, o = {}) => ({
	key,
	code,
	metaKey: false,
	ctrlKey: false,
	altKey: false,
	shiftKey: false,
	...o,
});
const cap = (e) => hyprFromKeyEvent(e);

// --- layout-aware: the character produced wins over the physical position ---
// AZERTY: the key labelled A sits at the physical Q position.
assert.deepEqual(cap(ev("a", "KeyQ", { metaKey: true })), { mods: ["SUPER"], key: "A" });
// Dvorak: physical KeyS produces "o".
assert.deepEqual(cap(ev("o", "KeyS", { metaKey: true })), { mods: ["SUPER"], key: "O" });
// Colemak: physical KeyE produces "f".
assert.equal(cap(ev("f", "KeyE")).key, "F");
// QWERTY is unchanged.
assert.equal(cap(ev("b", "KeyB", { metaKey: true })).key, "B");
// Caps Lock / Shift producing an upper-case char gives the same bind.
assert.equal(cap(ev("B", "KeyB", { shiftKey: true })).key, "B");
assert.deepEqual(cap(ev("B", "KeyB", { shiftKey: true, metaKey: true })).mods, ["SUPER", "SHIFT"]);

// --- digits and shifted symbols ---
assert.equal(cap(ev("1", "Digit1", { metaKey: true })).key, "1");
// Shift+1 on US produces "!": bind is SHIFT + 1, not "!".
assert.deepEqual(cap(ev("!", "Digit1", { metaKey: true, shiftKey: true })), {
	mods: ["SUPER", "SHIFT"],
	key: "1",
});
assert.equal(cap(ev("{", "BracketLeft", { shiftKey: true })).key, "bracketleft");
assert.equal(cap(ev("?", "Slash", { shiftKey: true })).key, "slash");

// --- punctuation as keysym names ---
assert.equal(cap(ev("[", "BracketLeft")).key, "bracketleft");
assert.equal(cap(ev("]", "BracketRight")).key, "bracketright");
assert.equal(cap(ev("/", "Slash")).key, "slash");
assert.equal(cap(ev(",", "Comma")).key, "comma");
assert.equal(cap(ev(" ", "Space")).key, "SPACE");
assert.equal(cap(ev("-", "Minus")).key, "minus");

// --- non-printable keys use names ---
assert.equal(cap(ev("Enter", "Enter")).key, "RETURN");
assert.equal(cap(ev("Escape", "Escape")).key, "ESCAPE");
assert.equal(cap(ev("Backspace", "Backspace")).key, "BACKSPACE");
assert.equal(cap(ev("Tab", "Tab")).key, "TAB");
assert.equal(cap(ev("ArrowLeft", "ArrowLeft")).key, "left");
assert.equal(cap(ev("PageDown", "PageDown")).key, "Next");
assert.equal(cap(ev("F11", "F11")).key, "F11");
assert.equal(cap(ev("AudioVolumeUp", "AudioVolumeUp")).key, "XF86AudioRaiseVolume");
assert.equal(cap(ev("Unidentified", "LaunchApp2")).key, "XF86Calculator");

// --- dead keys fall back to the physical key ---
assert.equal(cap(ev("Dead", "KeyE")).key, "E");
assert.equal(cap(ev("Dead", "Digit6")).key, "6");

// --- other characters become xkb Uxxxx names ---
assert.equal(cap(ev("é", "Digit2")).key, "U00E9");

// --- bare modifiers and unknown keys are not a capture yet ---
for (const m of ["Shift", "Control", "Alt", "Meta", "AltGraph"]) assert.equal(cap(ev(m, "ShiftLeft")), null);
assert.equal(cap(ev("Unidentified", "Foo")), null);

// --- conflicts ---
assert.deepEqual(normalizeMods(["shift", "Super", "SUPER"]), ["SHIFT", "SUPER"]);
assert.deepEqual(normalizeMods(["MOD4", "Control"]), ["CTRL", "SUPER"]);
assert.equal(comboId(["SHIFT", "SUPER"], "Q"), comboId(["super", "shift"], "q"));
assert.deepEqual(effectiveMods(undefined, ["SUPER"]), ["SUPER"]);
assert.deepEqual(effectiveMods([], ["SUPER"]), []);

const binds = [
	{ name: "browser", key: "B" }, // default mods (SUPER)
	{ name: "move left", key: "h", mods: ["SUPER", "SHIFT"] },
	{ name: "volume up", key: "XF86AudioRaiseVolume", mods: [] },
	{ name: "unset", key: "" },
];
const def = ["SUPER"];
assert.equal(findConflict(binds, { mods: ["SUPER"], key: "b" }, def)?.name, "browser");
assert.equal(findConflict(binds, { mods: ["SHIFT", "SUPER"], key: "H" }, def)?.name, "move left");
assert.equal(findConflict(binds, { mods: [], key: "XF86AudioRaiseVolume" }, def)?.name, "volume up");
// Same key, different modifiers: no conflict.
assert.equal(findConflict(binds, { mods: ["SUPER", "SHIFT"], key: "B" }, def), undefined);
// The bind being edited never conflicts with itself.
assert.equal(findConflict(binds, { mods: ["SUPER"], key: "B" }, def, binds[0]), undefined);
// Explicitly empty mods differ from the default mods.
assert.equal(findConflict(binds, { mods: ["SUPER"], key: "XF86AudioRaiseVolume" }, def), undefined);
// Binds with no key never conflict.
assert.equal(findConflict(binds, { mods: ["SUPER"], key: "" }, def), undefined);

console.log("keycapture.test.mjs: ok");
