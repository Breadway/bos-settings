// Run with: node --experimental-strip-types frontend/src/lib/nightlight.test.mjs
import assert from "node:assert/strict";
import { kelvinToWarmth, warmthToKelvin, MIN_KELVIN, MAX_KELVIN } from "./nightlight.ts";

// Right (more warmth) is a lower Kelvin value.
assert.equal(warmthToKelvin(0), MAX_KELVIN);
assert.equal(warmthToKelvin(100), MIN_KELVIN);
assert.ok(warmthToKelvin(80) < warmthToKelvin(20));

// Monotonic: every step toward warm never raises the Kelvin value.
let prev = Infinity;
for (let w = 0; w <= 100; w++) {
	const k = warmthToKelvin(w);
	assert.ok(k <= prev, `warmth ${w} -> ${k}K rose above ${prev}K`);
	prev = k;
}

// Round trip within one slider step (100K is 2.2 warmth points).
for (const k of [2000, 2700, 3500, 4500, 6500]) {
	assert.ok(Math.abs(warmthToKelvin(kelvinToWarmth(k)) - k) <= 100, `round trip ${k}`);
}

// Out-of-range input is clamped, never extrapolated.
assert.equal(kelvinToWarmth(100), 100);
assert.equal(kelvinToWarmth(99999), 0);
assert.equal(warmthToKelvin(-5), MAX_KELVIN);
assert.equal(warmthToKelvin(500), MIN_KELVIN);

console.log("nightlight.test.mjs: ok");
