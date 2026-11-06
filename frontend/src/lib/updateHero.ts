// Which headline the Updates page shows. A failed check is never reported as
// "up to date" (BSET-71): zero updates from a check that errored means unknown.
import { updateCounts } from "./updateCount.ts";

export interface UpdateCheck {
	total?: number;
	pacman: unknown[];
	pacman_error: string | null;
	bakery: unknown[];
	bakery_error: string | null;
	firmware?: unknown[];
}

export type Hero = "updating" | "checking" | "ready" | "failed" | "firmware" | "current";

export function heroState(status: UpdateCheck | null, busy: boolean): Hero {
	if (busy) return "updating";
	if (!status) return "checking";
	const counts = updateCounts(status);
	if (counts.total > 0) return "ready";
	if (status.pacman_error || status.bakery_error) return "failed";
	if (counts.firmware > 0) return "firmware";
	return "current";
}

/** Which parts could not be checked, for the failed headline. */
export function failedParts(status: UpdateCheck): string {
	const parts = [status.pacman_error && "system packages", status.bakery_error && "bread apps"].filter(Boolean);
	return parts.join(" and ");
}
