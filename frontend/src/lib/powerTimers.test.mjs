// Run with: node --experimental-strip-types frontend/src/lib/powerTimers.test.mjs
import assert from "node:assert/strict";
import { timerChoices, timerLabel } from "./powerTimers.ts";

assert.equal(timerLabel("0"), "Never");
assert.equal(timerLabel("60"), "1 minute");
assert.equal(timerLabel("300"), "5 minutes");
assert.equal(timerLabel("470"), "7 minutes 50 seconds");
assert.equal(timerLabel("3600"), "1 hour");

// Presets are present, including the 5 and 10 minute choices.
const preset = timerChoices(470);
assert.ok(preset.options.includes("300"), "5 minutes is a preset");
assert.ok(preset.options.includes("600"), "10 minutes is a preset");
assert.equal(preset.labels["300"], "5 minutes");
assert.equal(preset.labels["600"], "10 minutes");
assert.equal(preset.labels["0"], "Never");

// The hypridle value is neither hidden nor silently rounded: it is present as
// the explicit current custom value and its seconds stay in the option value.
assert.ok(preset.options.includes("470"), "the current 470 s value stays selectable");
assert.equal(preset.labels["470"], "Custom: 7 minutes 50 seconds (current)");

// A value already covered by a preset is not labelled custom.
const presetValue = timerChoices(300);
assert.equal(presetValue.labels["300"], "5 minutes");
assert.ok(!presetValue.options.some((value) => presetValue.labels[value].includes("Custom")));

// Any other odd value round-trips the same way (the old hard-coded list had
// 480 and 510 as presets, but existing configs can hold anything).
const odd = timerChoices(330);
assert.equal(odd.labels["330"], "Custom: 5 minutes 30 seconds (current)");
assert.ok(odd.options.includes("330"));

console.log("powerTimers ok");
