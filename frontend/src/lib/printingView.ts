// Which body the Printers page shows. A healthy CUPS with no queues is an
// empty list (BSET-90), not a broken service; only cups_ok false is an error.

export interface PrintingStatusLike {
	printers: unknown[];
	cups_ok: boolean;
	error: string | null;
}

export type PrintingState = "loading" | "error" | "empty" | "list";

export function printingState(status: PrintingStatusLike | null): PrintingState {
	if (!status) return "loading";
	if (!status.cups_ok) return "error";
	return status.printers.length === 0 ? "empty" : "list";
}

/** A useful error for the red banner, even when lpstat printed nothing. */
export function printingError(status: PrintingStatusLike): string {
	return status.error?.trim() || "Check that CUPS is installed and running.";
}
