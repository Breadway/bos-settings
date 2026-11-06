// Run with: node --conditions=browser --experimental-strip-types frontend/src/lib/firmwareDeviceList.test.mjs
//
// BSET-95: fwupd Names are human labels and can repeat across identical
// devices. Keying the firmware rows by Name made Svelte 5 throw
// each_key_duplicate in the client runtime and took the Updates page down.
// This mounts the real component with two same-name devices.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { JSDOM } from "jsdom";
import { compile } from "svelte/compiler";

const dom = new JSDOM("<!doctype html><html><body></body></html>", { pretendToBeVisual: true });
for (const name of [
	"window",
	"document",
	"Node",
	"Element",
	"DocumentFragment",
	"Text",
	"Comment",
	"HTMLElement",
	"SVGElement",
	"Event",
	"CustomEvent",
	"requestAnimationFrame",
	"cancelAnimationFrame",
	"getComputedStyle"
]) {
	if (dom.window[name] !== undefined) globalThis[name] = dom.window[name];
}
globalThis.window = dom.window;

const filename = "FirmwareDeviceList.svelte";
const source = await readFile(new URL("./components/FirmwareDeviceList.svelte", import.meta.url), "utf8");
const { js } = compile(source, { filename, generate: "client", dev: true });
const runtime = import.meta.resolve("svelte/internal/client");
const disclose = import.meta.resolve("svelte/internal/disclose-version");
const code = js.code
	.replaceAll("svelte/internal/client", runtime)
	.replaceAll("svelte/internal/disclose-version", disclose);
const component = await import(
	`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`
);
const { flushSync, mount, unmount } = await import("svelte");

const target = document.createElement("div");
document.body.appendChild(target);
const devices = [
	{ name: "USB-C Dock", version: "1.0" },
	{ name: "USB-C Dock", version: "2.0" }
];

const app = mount(component.default, { target, props: { devices } });
flushSync();

assert.equal((target.textContent.match(/USB-C Dock/g) ?? []).length, 2, "both same-name devices render");
assert.ok(target.textContent.includes("Version 1.0"), "first device version renders");
assert.ok(target.textContent.includes("Version 2.0"), "second device version renders");

unmount(app);
console.log("firmwareDeviceList: two same-name devices mount without a duplicate key");
