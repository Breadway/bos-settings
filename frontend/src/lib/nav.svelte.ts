import { guardNavigation } from "$lib/drafts.svelte";
import { DEFAULT_PAGE, resolveDestination } from "$lib/sidebar";

/** Shared navigation  -  module `$state` so sidebar/search/hubs all see the same page. */
export const nav = $state({
	page: DEFAULT_PAGE,
	tab: null as string | null,
	searchNonce: 0,
	focusLabel: "",
});

export function go(page: string, tab?: string, target = "") {
	const resolved = resolveDestination(page, tab);
	guardNavigation(() => {
		nav.tab = resolved.tab ?? "";
		nav.page = resolved.page;
		nav.focusLabel = target;
	});
}

export type Navigate = (page: string, tab?: string) => void;
export const NAVIGATE_KEY = "bos-settings-navigate";
