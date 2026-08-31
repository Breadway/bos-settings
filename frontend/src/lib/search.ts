export interface SearchHit {
	page: string;
	tab?: string;
	label: string;
	keywords: string;
}

export const SEARCH_INDEX: SearchHit[] = [
	{ page: "home", label: "Home", keywords: "search overview status" },
	{
		page: "network",
		label: "Wi-Fi",
		keywords: "wifi ssid password scan radio ethernet hotspot dns",
	},
	{
		page: "network",
		tab: "vpn",
		label: "VPN / WireGuard",
		keywords: "vpn wireguard tailscale tunnel import conf",
	},
	{
		page: "network",
		tab: "breadcrumbs",
		label: "Saved networks",
		keywords: "wifi profiles breadcrumbs known ssid tailscale",
	},
	{ page: "bluetooth", label: "Bluetooth", keywords: "bluetooth headphones pair scan mouse keyboard" },
	{
		page: "displays",
		label: "Displays",
		keywords: "monitor scale resolution arrange rotate vrr hdr hyprland",
	},
	{
		page: "displays",
		tab: "nightlight",
		label: "Night light",
		keywords: "night light hyprsunset temperature warmth sunset",
	},
	{
		page: "displays",
		tab: "breadmon",
		label: "Live arrange",
		keywords: "breadmon mirror profile arrange",
	},
	{
		page: "appearance",
		label: "Wallpaper",
		keywords: "wallpaper theme palette pywal breadpaper accent",
	},
	{
		page: "appearance",
		tab: "appearance",
		label: "Windows",
		keywords: "gaps blur rounding border shadow tiling dwindle",
	},
	{ page: "sound", label: "Sound", keywords: "volume mute microphone output input speaker pavucontrol" },
	{
		page: "power",
		label: "Power & battery",
		keywords: "battery brightness charge tlp suspend idle lid",
	},
	{
		page: "input",
		label: "Keyboard & mouse",
		keywords: "keybind shortcut layout touchpad tap natural scroll follow mouse",
	},
	{ page: "input", tab: "ime", label: "Input method", keywords: "fcitx5 ime cjk chinese japanese korean" },
	{
		page: "input",
		tab: "accessibility",
		label: "Accessibility",
		keywords: "orca screen reader zoom magnifier sticky keys",
	},
	{
		page: "desktop",
		label: "Bar",
		keywords: "breadbar modules clock tray workspaces",
	},
	{ page: "desktop", tab: "breadbox", label: "Launcher", keywords: "breadbox launcher apps" },
	{ page: "desktop", tab: "breadlock", label: "Lock screen", keywords: "lock greet breadlock idle" },
	{ page: "desktop", tab: "breadshot", label: "Screenshots", keywords: "screenshot grim slurp breadshot" },
	{ page: "desktop", tab: "autostart", label: "Startup apps", keywords: "autostart login startup" },
	{ page: "desktop", tab: "breadclip", label: "Clipboard", keywords: "clipboard history breadclip" },
	{ page: "desktop", tab: "breadpad", label: "Notes", keywords: "breadpad notes calendar caldav" },
	{ page: "desktop", tab: "breadsearch", label: "File search", keywords: "breadsearch breadmill index" },
	{ page: "desktop", tab: "bread", label: "Daemon", keywords: "breadd daemon adapters lua" },
	{ page: "apps", label: "Default apps", keywords: "browser terminal pdf mime default" },
	{ page: "apps", tab: "optional", label: "Optional software", keywords: "steam flatpak libreoffice" },
	{ page: "apps", tab: "printing", label: "Printing", keywords: "cups printer" },
	{ page: "privacy", label: "Firewall", keywords: "firewall ufw port allow" },
	{ page: "privacy", tab: "users", label: "Users", keywords: "users password account" },
	{ page: "system", label: "Updates", keywords: "update bakery pacman firmware nvidia" },
	{ page: "system", tab: "packages", label: "Packages", keywords: "packages bakery install" },
	{ page: "system", tab: "aur", label: "AUR", keywords: "aur yay" },
	{ page: "system", tab: "snapshots", label: "Snapshots", keywords: "snapper snapshot rollback grub" },
	{ page: "system", tab: "backup", label: "Backup", keywords: "restic backup restore" },
	{ page: "system", tab: "channel", label: "Bakery channel", keywords: "track stable beta dev bakery" },
	{ page: "about", label: "About", keywords: "hostname kernel cpu gpu about machine" },
	{ page: "about", tab: "datetime", label: "Date & time", keywords: "timezone ntp clock date time" },
	{ page: "desktop", tab: "breadhelp", label: "Help", keywords: "help breadhelp onboarding" },
];

export function searchSettings(query: string, limit = 8): SearchHit[] {
	const q = query.trim().toLowerCase();
	if (q.length < 1) return [];
	const scored = SEARCH_INDEX.map((hit) => {
		const hay = `${hit.label} ${hit.keywords} ${hit.page}`.toLowerCase();
		let score = 0;
		if (hit.label.toLowerCase().startsWith(q)) score += 8;
		if (hit.label.toLowerCase().includes(q)) score += 4;
		if (hay.includes(q)) score += 2;
		for (const part of q.split(/\s+/)) {
			if (hay.includes(part)) score += 1;
		}
		return { hit, score };
	})
		.filter((x) => x.score > 0)
		.sort((a, b) => b.score - a.score);
	return scored.slice(0, limit).map((x) => x.hit);
}
