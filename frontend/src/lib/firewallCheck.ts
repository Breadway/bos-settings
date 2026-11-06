/** Plain-language state for the firewall page before any privileged check. */
export function firewallNotCheckedMessage(serviceActive: boolean | null): string {
	if (serviceActive === true) return "The firewall is on. Rule details haven't been checked yet.";
	if (serviceActive === false) return "The firewall is off. Rule details haven't been checked yet.";
	return "Not checked yet.";
}

export interface FirewallFailure {
	declined: boolean;
	message: string;
}

/**
 * Turn a failed privileged check into plain language. A dismissed polkit
 * prompt is not an error the user needs to read raw; it is simply "not
 * checked". Anything else is a real failure worth showing.
 */
export function firewallCheckFailure(raw: string): FirewallFailure {
	const text = String(raw ?? "");
	const s = text.toLowerCase();
	const declined =
		s.includes("request dismissed") ||
		s.includes("dismissed") ||
		s.includes("not authorized") ||
		(s.includes("authentication") && (s.includes("cancel") || s.includes("dismiss"))) ||
		s.includes("cancelled") ||
		s.includes("canceled");
	if (declined) {
		return {
			declined: true,
			message: "Firewall status not checked. You cancelled the password request.",
		};
	}
	return {
		declined: false,
		message: text.trim() || "The firewall didn't answer.",
	};
}
