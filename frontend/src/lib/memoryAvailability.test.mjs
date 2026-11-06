// Run with: node --experimental-strip-types frontend/src/lib/memoryAvailability.test.mjs
import assert from "node:assert/strict";
import { memoryAvailability } from "./memoryAvailability.ts";

assert.equal(memoryAvailability(null), "checking");

// A machine without breadram shows the not-installed state and no controls.
assert.equal(memoryAvailability(false), "not-installed");

// An installed daemon keeps the full page, whether or not it is running now.
assert.equal(memoryAvailability(true), "available");

console.log("memoryAvailability ok");
