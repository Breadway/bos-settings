// Run with: node --experimental-strip-types frontend/src/lib/displayLayout.test.mjs
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { groupMonitors, groupOf, mirrorChoices, withMirror, withPosition, withMode, boxW, boxH } from "./displayLayout.ts";

// Same shape the Rust backend produces from the real `hyprctl monitors all -j` capture.
const raw = JSON.parse(
	readFileSync(new URL("../../../src/src/commands/testdata/hyprctl-monitors-all.json", import.meta.url), "utf8"),
);
const byId = new Map(raw.map((m) => [String(m.id), m.name]));
const layout = raw.map((m) => ({
	name: m.name,
	mode: "",
	x: m.x,
	y: m.y,
	width: m.width,
	height: m.height,
	refresh: m.refreshRate,
	scale: m.scale,
	transform: m.transform,
	available_modes: m.availableModes,
	vrr: 0,
	mirror_of: m.mirrorOf === "none" ? "" : (byId.get(m.mirrorOf) ?? m.mirrorOf),
}));

// DVI-I-1 mirrors eDP-1 and is drawn as one group with it; DVI-I-2 is separate.
const groups = groupMonitors(layout);
assert.equal(groups.length, 2);
const edp = groups.find((g) => g.source.name === "eDP-1");
assert.deepEqual(edp.mirrors.map((m) => m.name), ["DVI-I-1"]);
assert.equal(groups.find((g) => g.source.name === "DVI-I-2").mirrors.length, 0);
assert.equal(groupOf(layout, "DVI-I-1").source.name, "eDP-1");

// Rotated output swaps its box.
const dvi2 = layout.find((m) => m.name === "DVI-I-2");
assert.deepEqual([boxW(dvi2), boxH(dvi2)], [1080, 1920]);

// Mirror choices: any separate display can pick another separate one; the source cannot mirror while something mirrors it.
assert.deepEqual(mirrorChoices(layout, "DVI-I-1"), ["eDP-1", "DVI-I-2"]);
assert.deepEqual(mirrorChoices(layout, "eDP-1"), []);
assert.deepEqual(mirrorChoices(layout, "DVI-I-2"), ["eDP-1"]);

// Moving an unrelated output keeps the mirror and does not touch the input.
const moved = withPosition(layout, "DVI-I-2", -2000, -500);
assert.equal(moved.find((m) => m.name === "DVI-I-1").mirror_of, "eDP-1");
assert.equal(moved.find((m) => m.name === "DVI-I-2").x, -2000);
assert.equal(layout.find((m) => m.name === "DVI-I-2").x, -1080);

// Resolution change on one output leaves the mirror alone.
const resized = withMode(layout, "DVI-I-2", "1280x720@60.00Hz");
assert.equal(resized.find((m) => m.name === "DVI-I-2").width, 1280);
assert.equal(resized.find((m) => m.name === "DVI-I-1").mirror_of, "eDP-1");

// Stop mirroring: the output leaves the group and lands beside the others, not on top.
const split = withMirror(layout, "DVI-I-1", "");
assert.equal(split.find((m) => m.name === "DVI-I-1").mirror_of, "");
assert.equal(groupMonitors(split).length, 3);
const placed = split.find((m) => m.name === "DVI-I-1");
assert.equal(placed.x, 1920);

// Start mirroring again.
const again = withMirror(split, "DVI-I-1", "eDP-1");
assert.equal(groupMonitors(again).length, 2);

// A mirror pointing at a missing output is shown as a separate display, never dropped.
const orphan = layout.map((m) => (m.name === "DVI-I-1" ? { ...m, mirror_of: "HDMI-A-9" } : m));
assert.equal(groupMonitors(orphan).length, 3);

console.log("displayLayout ok");
