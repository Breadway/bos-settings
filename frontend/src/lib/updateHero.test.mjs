import assert from "node:assert/strict";
import { heroState, failedParts } from "./updateHero.ts";

const ok = { total: 0, pacman: [], pacman_error: null, bakery: [], bakery_error: null, firmware: [] };
assert.equal(heroState(null, false), "checking");
assert.equal(heroState(ok, true), "updating");
assert.equal(heroState(ok, false), "current");
assert.equal(heroState({ ...ok, pacman_error: "database file for 'core' does not exist" }, false), "failed", "BSET-71: a failed check is not up to date");
assert.equal(heroState({ ...ok, bakery_error: "offline" }, false), "failed");
assert.equal(heroState({ ...ok, pacman: [{}], total: 1, pacman_error: null, bakery_error: "offline" }, false), "ready", "known updates still offered");
assert.equal(failedParts({ ...ok, pacman_error: "x" }), "system packages");
assert.equal(failedParts({ ...ok, pacman_error: "x", bakery_error: "y" }), "system packages and bread apps");

for (const count of [0, 1, 10]) {
	const rows = Array.from({ length: count }, () => ({}));
	const status = { ...ok, total: count, pacman: rows };
	assert.equal(heroState(status, false), count === 0 ? "current" : "ready", `hero for ${count} package updates`);
}
assert.equal(heroState({ ...ok, firmware: [{}, {}] }, false), "firmware", "firmware-only state is labelled, not counted as packages");
console.log("updateHero: a failed check never reads as up to date; 0, 1 and 10 package updates agree");
