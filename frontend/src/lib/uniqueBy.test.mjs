// Run with: node --experimental-strip-types frontend/src/lib/uniqueBy.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { uniqueBy } from "./uniqueBy.ts";

assert.deepEqual(uniqueBy(["b", "a", "b", "c", "a"], (s) => s), ["b", "a", "c"]);
assert.deepEqual(uniqueBy([], (s) => s), []);
assert.deepEqual(uniqueBy([{ n: "x", v: 1 }, { n: "x", v: 2 }], (o) => o.n), [{ n: "x", v: 1 }]);

// The real DVI capture repeats five modes: the picker list must come out unique and in order.
const fixture = new URL("../../../src/src/commands/testdata/hyprctl-monitors-all.json", import.meta.url);
const monitors = JSON.parse(readFileSync(fixture, "utf8"));
for (const m of monitors) {
	const modes = uniqueBy(m.availableModes, (s) => s);
	assert.equal(new Set(modes).size, modes.length, m.name);
	assert.equal(modes.length, m.name === "eDP-1" ? 11 : 25, m.name);
	assert.equal(modes[0], m.availableModes[0], m.name);
}
console.log("uniqueBy ok");
