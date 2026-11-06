import assert from "node:assert/strict";
import { updateCounts } from "./updateCount.ts";

function fake(packages, firmware = 0, total = packages) {
	const rows = Array.from({ length: packages }, (_, index) => ({ name: `pkg${index}` }));
	return {
		total,
		pacman: rows,
		bakery: [],
		firmware: Array.from({ length: firmware }, (_, index) => ({ name: `fw${index}` })),
		pacman_error: null,
		bakery_error: null
	};
}

for (const count of [0, 1, 10]) {
	const counts = updateCounts(fake(count, 9));
	assert.equal(counts.total, count, `total for ${count} package updates`);
	assert.equal(counts.system, count);
	assert.equal(counts.firmware, 9, "firmware stays separate from package count");
}

assert.equal(updateCounts(null).total, 0);
assert.equal(
	updateCounts({ pacman: [{}], bakery: [{}], firmware: [{}, {}, {}] }).total,
	2,
	"fallback counts packages, not firmware devices"
);
assert.equal(
	updateCounts({ total: 3, pacman: [{}], bakery: [{}], firmware: [{}, {}, {}] }).total,
	3,
	"the backend total is authoritative"
);

console.log("updateCount: 0, 1 and 10 package updates agree; firmware stays separate");
