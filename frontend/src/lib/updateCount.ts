/**
 * One package update count for the sidebar badge, Home and the Updates page.
 *
 * `get_updates_status` computes `total` from the pacman and bakery lists.
 * Firmware devices are deliberately not part of it: a device that can accept
 * firmware updates is not a pending package update, and counting it made the
 * badge say ten while the Updates page listed one.
 */
export interface UpdateStatusLike {
	total?: number;
	pacman: unknown[];
	bakery: unknown[];
	firmware?: unknown[];
}

export interface UpdateCounts {
	total: number;
	system: number;
	bread: number;
	firmware: number;
}

export function updateCounts(status: UpdateStatusLike | null): UpdateCounts {
	if (!status) return { total: 0, system: 0, bread: 0, firmware: 0 };
	const system = status.pacman.length;
	const bread = status.bakery.length;
	return {
		total: status.total ?? system + bread,
		system,
		bread,
		firmware: status.firmware?.length ?? 0
	};
}
