// Run with: node --experimental-strip-types frontend/src/lib/printingView.test.mjs
import assert from "node:assert/strict";
import { printingState, printingError } from "./printingView.ts";

const healthyEmpty = { printers: [], cups_ok: true, error: null };
const healthyList = { printers: [{ name: "Office" }], cups_ok: true, error: null };
const broken = { printers: [], cups_ok: false, error: "lpstat: Connection refused" };

assert.equal(printingState(null), "loading");
assert.equal(printingState(healthyEmpty), "empty", "BSET-90: a healthy printerless system is not an error");
assert.equal(printingState(healthyList), "list");
assert.equal(printingState(broken), "error");
assert.equal(printingError(broken), "lpstat: Connection refused");
assert.equal(printingError({ printers: [], cups_ok: false, error: null }), "Check that CUPS is installed and running.");
assert.equal(printingError({ printers: [], cups_ok: false, error: "  " }), "Check that CUPS is installed and running.");
console.log("printingView: no printers is an empty state, only a failed service is an error");
