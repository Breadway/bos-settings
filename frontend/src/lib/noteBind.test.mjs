// Run with: node --experimental-strip-types frontend/src/lib/noteBind.test.mjs
import assert from "node:assert/strict";
import { findNoteBind, noteShortcutText } from "./noteBind.ts";

const file = (bindings, extra = {}) => ({ default_mods: ["SUPER"], bindings, ...extra });

// BOS's shipped flat schema: breadpad on U with the default modifier.
assert.equal(
	noteShortcutText(
		file([
			{ action: "exec", command: "breadbar --history", key: "N", mods: ["SUPER", "SHIFT"] },
			{ action: "exec", command: "breadpad", key: "U", label: "Notes / reminders (breadpad)" },
		]),
	),
	"Super + U",
);

// A moved bind is read from the file, not remembered.
assert.equal(noteShortcutText(file([{ action: "exec", command: "breadpad", key: "P", mods: ["CTRL"] }])), "Ctrl + P");

// An explicit empty mods list stays empty instead of inheriting default_mods.
assert.equal(noteShortcutText(file([{ action: "exec", command: "breadpad", key: "F9", mods: [] }])), "F9");

// Shell wrapping and absolute paths still identify breadpad.
assert.equal(noteShortcutText(file([{ action: "exec", command: "sh -c 'breadpad'", key: "U" }])), "Super + U");
assert.equal(noteShortcutText(file([{ action: "exec", command: "/usr/bin/breadpad", key: "U" }])), "Super + U");

// A capture-specific breadpad bind wins over a plain launcher bind.
assert.equal(
	noteShortcutText(
		file([
			{ action: "exec", command: "breadpad", key: "U" },
			{ action: "exec", command: "breadpad --capture", key: "Y" },
		]),
	),
	"Super + Y",
);

// Multi-layout files are searched too.
assert.equal(
	noteShortcutText({
		default_mods: ["SUPER"],
		common: [{ action: "exec", command: "breadpad", key: "U" }],
	}),
	"Super + U",
);

// No bind at all: fall back to copy that does not name a key.
assert.equal(noteShortcutText(file([])), null);
assert.equal(noteShortcutText(file([{ action: "exec", command: "breadbar --history", key: "N" }])), null);

// A bind without a key cannot produce a shortcut.
assert.equal(noteShortcutText(file([{ action: "exec", command: "breadpad" }])), null);

// Unreadable or missing payloads do not throw.
assert.equal(noteShortcutText(null), null);
assert.equal(noteShortcutText(undefined), null);
assert.equal(noteShortcutText({}), null);
assert.equal(noteShortcutText({ bindings: null }), null);
assert.equal(findNoteBind(null), null);

// Non-exec breadpad actions are not the capture bind.
assert.equal(noteShortcutText(file([{ action: "focus", command: "breadpad", key: "U" }])), null);

console.log("noteBind: all assertions passed");
