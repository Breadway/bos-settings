import { invoke } from "@tauri-apps/api/core";

/**
 * breadcrumbs (Wi-Fi places) config shared by the Wi-Fi page's sections:
 * Places, saved networks and the advanced settings all edit this one object,
 * and the Wi-Fi section registers the single draft that saves it.
 */
export interface CrumbSettings {
	default_profile: string;
	dns: string;
	exit_node: string;
	ping_host: string;
	connectivity_url: string;
	nmcli_wait: number;
	watch_interval: number;
}
export interface SavedNetwork {
	ssid: string;
	/** Write-only: never returned by the backend. Empty keeps the stored one. */
	password?: string | null;
	hidden: boolean;
}
export interface Place {
	name: string;
	networks: string[];
	detect_ssids: string[];
	bootstrap: string;
	exit_node: string;
	tailscale: boolean;
	include_all_known: boolean;
}
export interface CrumbConfig {
	settings: CrumbSettings;
	networks: SavedNetwork[];
	profiles: Place[];
}

class CrumbStore {
	cfg = $state<CrumbConfig | null>(null);
	/** The place breadcrumbs is using now, when it is installed. */
	current = $state<string | null>(null);
}
export const crumbs = new CrumbStore();

export async function loadCrumbs() {
	const [loaded, current] = await Promise.all([
		invoke<CrumbConfig>("get_breadcrumbs_config"),
		invoke<string | null>("get_current_place").catch(() => null),
	]);
	loaded.networks = loaded.networks.map((n) => ({ ...n, password: "" }));
	crumbs.cfg = loaded;
	crumbs.current = current;
}

export async function saveCrumbs() {
	const cfg = crumbs.cfg!;
	// Places are keyed by name in the file, so a blank or repeated name
	// would silently drop a place. Refuse the save instead.
	const seen = new Set<string>();
	for (const p of cfg.profiles) {
		const name = p.name.trim();
		if (!name) throw new Error("Every place needs a name.");
		if (seen.has(name)) throw new Error(`Two places are called ${name}. Rename one of them.`);
		seen.add(name);
	}
	await invoke("save_breadcrumbs_config", {
		input: {
			...cfg,
			networks: cfg.networks.map((n) => ({
				ssid: n.ssid,
				hidden: n.hidden,
				...(n.password ? { password: n.password } : {}),
			})),
		},
	});
	// A typed password is sent once; clearing it stops it looking stored.
	for (const n of cfg.networks) n.password = "";
}

/** Older breadcrumbs created lowercase home/work/away places even when the
 *  user had their own. An empty one that is neither in use nor the default
 *  is clutter the page offers to tidy up. */
export function isLeftoverPlace(p: Place, cfg: CrumbConfig, current: string | null): boolean {
	if (!["home", "work", "away"].includes(p.name)) return false;
	if (p.networks.length || p.detect_ssids.length || p.bootstrap || p.exit_node || p.tailscale) return false;
	if (p.name === cfg.settings.default_profile || p.name === current) return false;
	return true;
}

function list(xs: string[]): string {
	const v = xs.filter(Boolean);
	return v.length <= 2 ? v.join(" or ") : `${v.slice(0, -1).join(", ")} or ${v[v.length - 1]}`;
}

/** One plain sentence describing what a place does. */
export function placeSentence(p: Place, globalExit: string): string {
	const parts: string[] = [];
	if (p.networks.length) parts.push(`Join ${list(p.networks)}${p.include_all_known ? ", or any other saved network" : ""}`);
	else if (p.include_all_known) parts.push("Join any saved network");
	else parts.push("No networks chosen yet");
	if (p.tailscale && p.bootstrap) parts.push(`sign in through ${p.bootstrap} first`);
	if (p.tailscale) {
		const node = p.exit_node || globalExit;
		parts.push(node ? `send traffic through ${node} with Tailscale` : "require Tailscale");
	}
	let s = parts.length > 1 ? `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}.` : `${parts[0]}.`;
	if (p.detect_ssids.length) s += ` Recognised when ${list(p.detect_ssids)} is nearby.`;
	return s;
}

/** breadcrumbs only uses a place's sign-in network and exit node when the
 *  place requires Tailscale; set without it they do nothing. */
export function unusedTailscaleBits(p: Place): string | null {
	if (p.tailscale) return null;
	const bits = [p.bootstrap && `sign in through ${p.bootstrap}`, p.exit_node && `use ${p.exit_node} as the exit node`].filter(Boolean);
	return bits.length ? `Set to ${bits.join(" and ")}, but Tailscale is off for this place, so that is skipped.` : null;
}
