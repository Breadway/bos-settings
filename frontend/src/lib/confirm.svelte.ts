// In-app replacement for window.confirm(). WebKitGTK in Tauri does not show
// script dialogs, so confirm() returned without asking and every destructive
// action ran unprompted (BSET-69). ConfirmHost renders the pending request.

export interface ConfirmRequest {
	title: string;
	message?: string;
	confirmLabel?: string;
	cancelLabel?: string;
	/** Defaults to true: the confirm button is styled as destructive. */
	danger?: boolean;
}

interface Pending extends ConfirmRequest {
	id: number;
	resolve: (ok: boolean) => void;
}

export const confirmState = $state({ pending: null as Pending | null });
let nextId = 0;

/** Resolves true only when the user presses the confirm button. */
export function confirmAction(req: ConfirmRequest | string): Promise<boolean> {
	const r = typeof req === "string" ? { title: req } : req;
	// A newer request cancels an open one instead of leaving its caller hanging.
	answer(false);
	return new Promise((resolve) => {
		confirmState.pending = { ...r, id: ++nextId, resolve };
	});
}

export function answer(ok: boolean) {
	const p = confirmState.pending;
	if (!p) return;
	confirmState.pending = null;
	p.resolve(ok);
}
