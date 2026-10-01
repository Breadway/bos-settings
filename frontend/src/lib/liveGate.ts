// Keeps a control that mirrors external state (volume, brightness) from
// fighting the user. Values read back from the system may replace what the
// page shows only when the user is not mid-drag and the read is not just the
// echo of the user's own last change.

export interface LiveGate {
	/** The user grabbed the control. */
	begin(): void;
	/** The user released the control. */
	end(): void;
	/** The user changed the value (keyboard, click). */
	touch(): void;
	/** May an externally read value overwrite what is shown? */
	canAdopt(): boolean;
}

export function createLiveGate(echoMs = 700, now: () => number = () => Date.now()): LiveGate {
	let editing = false;
	let lastLocal = -Infinity;
	return {
		begin() {
			editing = true;
		},
		end() {
			editing = false;
			lastLocal = now();
		},
		touch() {
			lastLocal = now();
		},
		canAdopt() {
			return !editing && now() - lastLocal > echoMs;
		},
	};
}

/**
 * Call `fn` every `ms` while the window is visible, and once right away when
 * it becomes visible again. Returns a cleanup function.
 */
export function pollWhileVisible(fn: () => void, ms: number): () => void {
	const tick = () => {
		if (typeof document === "undefined" || document.visibilityState === "visible") fn();
	};
	const id = setInterval(tick, ms);
	const onVis = () => tick();
	if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVis);
	return () => {
		clearInterval(id);
		if (typeof document !== "undefined") document.removeEventListener("visibilitychange", onVis);
	};
}
