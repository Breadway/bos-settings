/**
 * Readable snapshot titles. snap-pac stores the pacman command line as the
 * "pre" description (e.g. "/usr/bin/pacman -U --noconfirm -- /tmp/foo-1.2-x86_64.pkg.tar.zst")
 * and the package names as the "post" one; neither reads well in a timeline.
 */
export function snapshotLabel(description: string, kind = ""): string {
	const text = description.trim();
	if (!text) return "";
	const words = text.split(/\s+/);
	const program = words[0].split("/").pop() ?? "";
	if (program === "pacman" || program === "yay" || program === "paru") {
		const ops = words.slice(1).filter((w) => w.startsWith("-") && !w.startsWith("--"));
		const targets = words.slice(1).filter((w) => !w.startsWith("-")).map(packageName).filter(Boolean);
		const flags = ops.join("");
		const verb = /R/.test(flags) ? "removing" : /S/.test(flags) && /u/.test(flags) && !targets.length ? "updating the system" : /S/.test(flags) && /u/.test(flags) ? "updating" : "installing";
		const list = summarize(targets);
		return `Before ${verb}${list ? ` ${list}` : ""}`;
	}
	if (kind === "post") return `After changing ${summarize(words.map(packageName).filter(Boolean))}`;
	return text;
}

function packageName(arg: string): string {
	const file = arg.split("/").pop() ?? arg;
	// local package file: name-version-release-arch.pkg.tar.zst
	const pkg = file.match(/^(.+?)-[^-]+-[^-]+-[^-]+\.pkg\.tar(?:\.\w+)?$/);
	if (pkg) return pkg[1];
	// snap-pac temp paths like codex-update-manager-484822-17910
	return file.replace(/(?:-\d+){1,3}$/, "");
}

function summarize(names: string[]): string {
	const unique = [...new Set(names)];
	if (unique.length <= 3) return unique.join(", ");
	return `${unique.slice(0, 3).join(", ")} and ${unique.length - 3} more`;
}
