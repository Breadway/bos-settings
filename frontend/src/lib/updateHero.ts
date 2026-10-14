// Which headline the Updates page shows. A failed check is never reported as
// "up to date" (BSET-71): zero updates from a check that errored means unknown.

export interface UpdateCheck {
	pacman: unknown[];
	pacman_error: string | null;
	bakery: unknown[];
	bakery_error: string | null;
}

export type Hero = "updating" | "checking" | "ready" | "failed" | "current";

export function heroState(status: UpdateCheck | null, busy: boolean): Hero {
	if (busy) return "updating";
	if (!status) return "checking";
	if (status.pacman.length + status.bakery.length > 0) return "ready";
	if (status.pacman_error || status.bakery_error) return "failed";
	return "current";
}

/** Which parts could not be checked, for the failed headline. */
export function failedParts(status: UpdateCheck): string {
	const parts = [status.pacman_error && "system packages", status.bakery_error && "bread apps"].filter(Boolean);
	return parts.join(" and ");
}
