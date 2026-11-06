import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { compileModule } from "svelte/compiler";
import { transformWithEsbuild } from "vite";

// BSET-69: window.confirm() is a no-op in the Tauri webview, so every
// destructive action ran unprompted. confirmAction must resolve false unless
// the user confirms, and no bare script dialog may come back.
const source = await readFile(new URL("./confirm.svelte.ts", import.meta.url), "utf8");
const stripped = await transformWithEsbuild(source, "confirm.svelte.ts", { loader: "ts" });
const compiled = compileModule(stripped.code, { filename: "confirm.svelte.js", generate: "client" });
const code = compiled.js.code.replace(/from ['"]([^'"]+)['"]/g, (_, s) => `from ${JSON.stringify(import.meta.resolve(s))}`);
const { confirmAction, confirmState, answer } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

assert.equal(confirmState.pending, null);
const yes = confirmAction({ title: "Remove x?", confirmLabel: "Remove" });
assert.equal(confirmState.pending?.title, "Remove x?", "a request is shown before anything runs");
answer(true);
assert.equal(await yes, true);
assert.equal(confirmState.pending, null);

const no = confirmAction("Delete?");
answer(false);
assert.equal(await no, false, "Cancel, Escape and backdrop resolve false");

const first = confirmAction("First?");
const second = confirmAction("Second?");
assert.equal(await first, false, "a superseded request is cancelled, not left pending");
assert.equal(confirmState.pending?.title, "Second?");
answer(true);
assert.equal(await second, true);
answer(true);
assert.equal(confirmState.pending, null, "answering with nothing pending is a no-op");

const root = fileURLToPath(new URL("..", import.meta.url));
const offenders = [];
async function walk(dir) {
	for (const e of await readdir(dir, { withFileTypes: true })) {
		const p = join(dir, e.name);
		if (e.isDirectory()) await walk(p);
		else if (/\.(svelte|ts)$/.test(e.name) && !/\.test\./.test(e.name)) {
			const lines = (await readFile(p, "utf8")).split("\n");
			lines.forEach((l, i) => {
				const code = l.replace(/\/\/.*$/, "");
				if (/(^|[^.\w])(window\.)?(confirm|alert|prompt)\s*\(/.test(code)) offenders.push(`${p}:${i + 1}`);
			});
		}
	}
}
await walk(root);
assert.deepEqual(offenders, [], "use confirmAction instead of window.confirm/alert/prompt");

// BSET-111: the bug-report page's destructive actions (queue discard on save,
// delete from server, local wipe, discard the queue) must ask through the
// shared dialog rather than a bare script dialog.
const bugReports = await readFile(new URL("./views/BugReports.svelte", import.meta.url), "utf8");
assert.match(
	bugReports,
	/import\s*\{[^}]*\bconfirmAction\b[^}]*\}\s*from\s*["']\$lib\/confirm\.svelte["']/,
	"BugReports uses the shared confirm dialog",
);
assert.ok(
	(bugReports.match(/await confirmAction\(/g) ?? []).length >= 4,
	"each BugReports destructive action asks before running",
);
console.log("confirm: in-app confirm resolves only on Confirm, and no script dialogs remain");
