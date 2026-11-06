// Run with: node --experimental-strip-types frontend/src/lib/greeting.test.mjs
import assert from "node:assert/strict";
import { greetingFor, greetingWithName } from "./greeting.ts";

assert.equal(greetingFor(9), "Good morning");
assert.equal(greetingFor(14), "Good afternoon");
assert.equal(greetingFor(21), "Good evening");
assert.equal(greetingFor(2), "Still up");

// The live image showed "Still up?, liveuser": the name goes before the
// question mark, not after a comma.
assert.equal(greetingWithName(2, "liveuser"), "Still up, liveuser?");
assert.equal(greetingWithName(2, ""), "Still up?");
assert.equal(greetingWithName(2, "   "), "Still up?");
assert.equal(greetingWithName(2, null), "Still up?");
assert.equal(greetingWithName(2, undefined), "Still up?");

// Daytime greetings take a comma and no trailing question mark.
assert.equal(greetingWithName(9, "liveuser"), "Good morning, liveuser");
assert.equal(greetingWithName(14, " liveuser "), "Good afternoon, liveuser");
assert.equal(greetingWithName(21, "liveuser"), "Good evening, liveuser");
assert.equal(greetingWithName(9, ""), "Good morning");
assert.equal(greetingWithName(9, null), "Good morning");

console.log("greeting ok");
