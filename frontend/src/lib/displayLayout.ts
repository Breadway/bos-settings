// Pure layout helpers for the Displays page: grouping mirrored outputs and
// building the layouts that get sent to the backend guard.

export interface LiveMonitor {
	name: string;
	mode: string;
	x: number;
	y: number;
	width: number;
	height: number;
	refresh: number;
	scale: number;
	transform: number;
	available_modes: string[];
	vrr: number;
	mirror_of: string;
	disabled?: boolean;
}

export interface MirrorGroup {
	/** The output that owns the picture and the position. */
	source: LiveMonitor;
	/** Outputs showing the same picture. */
	mirrors: LiveMonitor[];
}

export function clone(list: readonly LiveMonitor[]): LiveMonitor[] {
	return list.map((m) => ({ ...m, available_modes: [...m.available_modes] }));
}

export function isRotated(m: LiveMonitor): boolean {
	return m.transform % 2 === 1;
}

export function boxW(m: LiveMonitor): number {
	return isRotated(m) ? m.height : m.width;
}

export function boxH(m: LiveMonitor): number {
	return isRotated(m) ? m.width : m.height;
}

/**
 * One group per picture. An output mirroring something that is not in the list
 * (or mirroring itself) is treated as a separate display rather than dropped.
 */
export function groupMonitors(list: readonly LiveMonitor[]): MirrorGroup[] {
	const names = new Set(list.map((m) => m.name));
	const isMirror = (m: LiveMonitor) => !!m.mirror_of && m.mirror_of !== m.name && names.has(m.mirror_of);
	const groups: MirrorGroup[] = list
		.filter((m) => !isMirror(m))
		.map((source) => ({ source, mirrors: [] as LiveMonitor[] }));
	for (const m of list.filter(isMirror)) {
		const group = groups.find((g) => g.source.name === m.mirror_of);
		if (group) group.mirrors.push(m);
		else groups.push({ source: m, mirrors: [] });
	}
	return groups;
}

export function groupOf(list: readonly LiveMonitor[], name: string): MirrorGroup | null {
	return groupMonitors(list).find((g) => g.source.name === name || g.mirrors.some((m) => m.name === name)) ?? null;
}

/** Names that `name` could mirror: separate displays other than itself, and only when nothing mirrors `name`. */
export function mirrorChoices(list: readonly LiveMonitor[], name: string): string[] {
	if (list.some((m) => m.mirror_of === name)) return [];
	return list.filter((m) => m.name !== name && !m.mirror_of).map((m) => m.name);
}

/** Start or stop mirroring. Everything else, other mirrors included, is left exactly as it is. */
export function withMirror(list: readonly LiveMonitor[], name: string, source: string): LiveMonitor[] {
	const next = clone(list);
	const m = next.find((o) => o.name === name);
	if (!m) return next;
	if (source) {
		m.mirror_of = source;
		return next;
	}
	m.mirror_of = "";
	// Stopping a mirror: put it beside the other screens, not on top of them.
	const others = next.filter((o) => o.name !== name && !o.mirror_of);
	m.x = others.reduce((edge, o) => Math.max(edge, o.x + boxW(o)), 0);
	m.y = others.length ? Math.min(...others.map((o) => o.y)) : 0;
	return next;
}

/** Move one separate display (or the source of a mirrored group). */
export function withPosition(list: readonly LiveMonitor[], name: string, x: number, y: number): LiveMonitor[] {
	const next = clone(list);
	const m = next.find((o) => o.name === name);
	if (m) {
		m.x = Math.round(x);
		m.y = Math.round(y);
	}
	return next;
}

/** "1920x1080@60.00Hz" -> { width, height, refresh }, or null when unreadable. */
export function parseMode(mode: string): { width: number; height: number; refresh: number } | null {
	const [res, hz] = mode.replace(/Hz$/i, "").split("@");
	const [w, h] = (res ?? "").split("x");
	const width = Number(w);
	const height = Number(h);
	const refresh = Number(hz);
	if (!width || !height) return null;
	return { width, height, refresh: refresh || 60 };
}

export function withMode(list: readonly LiveMonitor[], name: string, mode: string): LiveMonitor[] {
	const next = clone(list);
	const m = next.find((o) => o.name === name);
	const parsed = parseMode(mode);
	if (m && parsed) {
		m.width = parsed.width;
		m.height = parsed.height;
		m.refresh = parsed.refresh;
		m.mode = `${parsed.width}x${parsed.height} @ ${Math.round(parsed.refresh)}Hz`;
	}
	return next;
}
