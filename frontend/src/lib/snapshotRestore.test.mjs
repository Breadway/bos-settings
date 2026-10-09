import assert from "node:assert/strict";
import { test } from "node:test";
import { confirmAndRestoreSnapshot } from "./snapshotRestore.ts";

test("cancel leaves the snapshot root untouched", async () => {
	let invoked = false;
	let message = "";
	await confirmAndRestoreSnapshot(
		"42",
		(value) => {
			message = value;
			return false;
		},
		async () => {
			invoked = true;
		},
		() => assert.fail("cancel must not report an error"),
	);
	assert.equal(invoked, false);
	assert.match(message, /current system is kept as a backup/);
	assert.match(message, /restarts into the system/);
});

test("confirmation passes the selected snapshot number to restore", async () => {
	let invokedWith = null;
	await confirmAndRestoreSnapshot(
		"42",
		(message) => message.includes("snapshot #42"),
		async (number) => {
			invokedWith = number;
		},
		() => assert.fail("successful restore must not report an error"),
	);
	assert.equal(invokedWith, "42");
});

test("restore failures are shown to the user", async () => {
	let errorMessage = "";
	await confirmAndRestoreSnapshot(
		"42",
		() => true,
		async () => {
			throw new Error("permission denied");
		},
		(message) => {
			errorMessage = message;
		},
	);
	assert.equal(errorMessage, "Error: permission denied");
});
