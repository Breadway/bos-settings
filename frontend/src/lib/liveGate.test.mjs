// Run with: node --experimental-strip-types frontend/src/lib/liveGate.test.mjs
import assert from "node:assert/strict";
import { createLiveGate } from "./liveGate.ts";

let t = 1_000_000;
const clock = () => t;

// Nothing happened locally: external values are adopted.
const g = createLiveGate(700, clock);
assert.equal(g.canAdopt(), true);

// Mid-drag: never adopt, however long it takes.
g.begin();
t += 10_000;
assert.equal(g.canAdopt(), false);

// Just released: the read-back is the echo of our own change, ignore it...
g.end();
assert.equal(g.canAdopt(), false);
t += 699;
assert.equal(g.canAdopt(), false);

// ...but a later external change (volume key) is picked up.
t += 2;
assert.equal(g.canAdopt(), true);

// A keyboard/click change counts as a local edit too.
g.touch();
assert.equal(g.canAdopt(), false);
t += 701;
assert.equal(g.canAdopt(), true);

console.log("liveGate.test.mjs: ok");
