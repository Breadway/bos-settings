import { onDestroy } from "svelte";
import { untrack } from "svelte";

type Draft = { dirty: boolean; save: () => Promise<void>; markSaved: (snapshot?: string) => void; snapshot: () => string };
export const drafts = $state({ items: [] as Draft[], pending: null as (() => void) | null, saving: false, error: "" });

/** Register explicit-save forms so both page and tab navigation can protect them. */
export function useDraft(read: () => unknown, save: () => Promise<void>, ready: () => boolean = () => read() != null) {
	let baseline = $state<string | undefined>();
	const draft: Draft = $state({
		dirty: false,
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
	drafts.items.push(draft);
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
