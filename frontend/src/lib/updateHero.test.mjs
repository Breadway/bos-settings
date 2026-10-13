import assert from "node:assert/strict";
import { heroState, failedParts } from "./updateHero.ts";

const ok = { pacman: [], pacman_error: null, bakery: [], bakery_error: null };
assert.equal(heroState(null, false), "checking");
assert.equal(heroState(ok, true), "updating");
assert.equal(heroState(ok, false), "current");
assert.equal(heroState({ ...ok, pacman_error: "database file for 'core' does not exist" }, false), "failed", "BSET-71: a failed check is not up to date");
assert.equal(heroState({ ...ok, bakery_error: "offline" }, false), "failed");
assert.equal(heroState({ ...ok, pacman: [{}], pacman_error: null, bakery_error: "offline" }, false), "ready", "known updates still offered");
assert.equal(failedParts({ ...ok, pacman_error: "x" }), "system packages");
assert.equal(failedParts({ ...ok, pacman_error: "x", bakery_error: "y" }), "system packages and bread apps");
console.log("updateHero: a failed check never reads as up to date");
