import { DEFAULT_PAGE, resolvePage } from "$lib/sidebar";

/** Shared navigation  -  module `$state` so sidebar/search/hubs all see the same page. */
export const nav = $state({
	page: DEFAULT_PAGE,
	tab: null as string | null,
	searchNonce: 0,
});

export function go(page: string, tab?: string) {
	const resolved = resolvePage(page);
	nav.tab = tab ?? resolved.tab ?? "";
	nav.page = resolved.page;
}

export type Navigate = (page: string, tab?: string) => void;
export const NAVIGATE_KEY = "bos-settings-navigate";
