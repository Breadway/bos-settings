import { onDestroy } from "svelte";
import { untrack } from "svelte";

type Draft = { sourceSave: () => Promise<void>; dirty: boolean; save: () => Promise<void>; markSaved: (snapshot?: string) => void; snapshot: () => string };

/**
 * `items` is a raw, immutably replaced array. A deep `$state` array that forms
 * `push` into while mounting and `filter` out of while unmounting picked up an
 * `undefined` hole during page swaps; the `.some(d => d.dirty)` in DraftNotice
 * then threw inside Svelte's flush and froze every later update (the next page
 * sat on "Loading..." forever). Each Draft is still its own `$state`, so
 * `dirty` stays reactive.
 */
class DraftStore {
	items = $state.raw<Draft[]>([]);
	pending = $state<(() => void) | null>(null);
	saving = $state(false);
	error = $state("");
}
export const drafts = new DraftStore();

/** Register explicit-save forms so both page and tab navigation can protect them. */
export function useDraft(read: () => unknown, save: () => Promise<void>, ready: () => boolean = () => read() != null) {
	let baseline = $state<string | undefined>();
	const draft: Draft = $state({
		dirty: false,
		sourceSave: save,
		snapshot: () => JSON.stringify(read()),
		markSaved: (snapshot?: string) => { baseline = snapshot ?? draft.snapshot(); draft.dirty = false; },
		save: async () => { await save(); draft.markSaved(); },
	});
	$effect(() => {
		if (!ready()) return;
		const snapshot = draft.snapshot();
		if (untrack(() => baseline) === undefined) baseline = snapshot;
		draft.dirty = snapshot !== baseline;
	});
	drafts.items = [...drafts.items, draft];
	onDestroy(() => { drafts.items = drafts.items.filter((d) => d !== draft); });
	return draft;
}

export function guardNavigation(action: () => void) {
	if (drafts.saving) return;
	if (drafts.items.some((d) => d.dirty)) { drafts.pending = action; drafts.error = ""; }
	else action();
}

export async function saveDrafts() {
	drafts.saving = true; drafts.error = "";
	try {
		for (const draft of drafts.items.filter((d) => d.dirty)) await draft.save();
		if (!drafts.items.some((d) => d.dirty)) { const action = drafts.pending; drafts.pending = null; action?.(); }
	} catch (e) { drafts.error = String(e); }
	finally { drafts.saving = false; }
}

export function discardAndContinue() {
	if (drafts.saving) return;
	const action = drafts.pending;
	drafts.pending = null;
	action?.();
}

/** A page save only clears the forms attached to that action. */
export function markActionSaved(save: () => Promise<void>) {
    for (const draft of drafts.items.filter(d => d.sourceSave === save)) draft.markSaved();
}
