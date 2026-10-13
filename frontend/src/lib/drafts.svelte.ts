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
	/** Shown briefly after an automatic save lands ("Saved"). */
	status = $state("");
}
export const drafts = new DraftStore();

/** Toggles, pickers and sliders save this long after the last change. */
const AUTOSAVE_MS = 450;
let statusTimer: ReturnType<typeof setTimeout> | undefined;

function editingText(): HTMLElement | null {
	if (typeof document === "undefined") return null;
	const el = document.activeElement;
	if (el instanceof HTMLTextAreaElement) return el;
	if (el instanceof HTMLInputElement && !["checkbox", "radio", "range", "button", "submit", "color", "file"].includes(el.type)) return el;
	return null;
}

function announceSaved() {
	drafts.status = "Saved";
	clearTimeout(statusTimer);
	statusTimer = setTimeout(() => (drafts.status = ""), 1800);
}

/**
 * Register a settings form. Every form saves itself: a change to a toggle,
 * picker or slider is written shortly after it is made, and a text field is
 * written when you leave it (so a half-typed password or path is never
 * saved). Leaving the page flushes anything still waiting.
 */
export function useDraft(read: () => unknown, save: () => Promise<void>, ready: () => boolean = () => read() != null) {
	let baseline = $state<string | undefined>();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let inflight: Promise<void> = Promise.resolve();
	const draft: Draft = $state({
		dirty: false,
		sourceSave: save,
		snapshot: () => JSON.stringify(read()),
		markSaved: (snapshot?: string) => { baseline = snapshot ?? draft.snapshot(); draft.dirty = false; },
		// Serialised: a second change while the first write is still running
		// waits for it instead of racing it to the file.
		save: () => {
			const run = inflight.then(async () => {
				const snapshot = draft.snapshot();
				await save();
				draft.markSaved(snapshot);
				drafts.error = "";
			});
			inflight = run.catch(() => {});
			return run;
		},
	});
	function autosave(delay = AUTOSAVE_MS) {
		clearTimeout(timer);
		timer = setTimeout(() => {
			if (!draft.dirty) return;
			const field = editingText();
			if (field) {
				field.addEventListener("blur", () => autosave(60), { once: true });
				return;
			}
			draft.save().then(announceSaved, (e) => { drafts.error = String(e); });
		}, delay);
	}
	$effect(() => {
		if (!ready()) return;
		const snapshot = draft.snapshot();
		if (untrack(() => baseline) === undefined) baseline = snapshot;
		draft.dirty = snapshot !== baseline;
		if (draft.dirty) untrack(() => autosave());
	});
	drafts.items = [...drafts.items, draft];
	onDestroy(() => {
		clearTimeout(timer);
		drafts.items = drafts.items.filter((d) => d !== draft);
		// A change made just before the section went away (a hub tab
		// switch, a window close) still gets written.
		if (draft.dirty) draft.save().then(announceSaved, (e) => { drafts.error = String(e); });
	});
	return draft;
}

/**
 * Page and section changes write any change still waiting first. Only a save
 * that fails stops the move, and the dialog then offers to retry or discard.
 */
export function guardNavigation(action: () => void) {
	if (drafts.saving) return;
	if (drafts.items.some((d) => d.dirty)) { drafts.pending = action; drafts.error = ""; void saveDrafts(); }
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
	drafts.error = "";
	action?.();
}

/** A page save only clears the forms attached to that action. */
export function markActionSaved(save: () => Promise<void>) {
    for (const draft of drafts.items.filter(d => d.sourceSave === save)) draft.markSaved();
}
