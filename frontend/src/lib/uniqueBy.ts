// Svelte throws each_key_duplicate (and the whole section fails to load) when
// a keyed {#each} sees the same key twice. Lists that come from the
// compositor are not guaranteed unique, so they pass through here first.
export function uniqueBy<T>(items: readonly T[], key: (item: T) => string): T[] {
	const seen = new Set<string>();
	const out: T[] = [];
	for (const item of items) {
		const k = key(item);
		if (seen.has(k)) continue;
		seen.add(k);
		out.push(item);
	}
	return out;
}
