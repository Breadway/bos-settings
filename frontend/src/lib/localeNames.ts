/** Readable names for glibc locale ids ("en_US.utf8" -> "English (United States)"). */
export function localeLabel(id: string, displayLocale = "en"): string {
	const base = id.split(".")[0].split("@")[0];
	if (!base || base === "C" || base === "POSIX") return id.startsWith("C.") || id === "C" ? "Computer default (C)" : id;
	try {
		const names = new Intl.DisplayNames([displayLocale], { type: "language", languageDisplay: "standard" });
		const name = names.of(base.replace("_", "-"));
		return name && name !== base ? name : id;
	} catch {
		return id;
	}
}

export function localeLabels(ids: string[], displayLocale = "en"): Record<string, string> {
	return Object.fromEntries(ids.map((id) => [id, localeLabel(id, displayLocale)]));
}
