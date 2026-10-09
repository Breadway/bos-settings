import { resolveDestination, SIDEBAR_SECTIONS } from "./sidebar";

export interface SearchHit {
	page: string;
	tab?: string;
	label: string;
	keywords: string;
	target?: string;
	context?: string;
}

export const SEARCH_INDEX: SearchHit[] = [
 { page: "storage", label:"Storage", keywords:"disk space cache clean trash downloads", target:"App caches" },
 { page: "sharing", label:"Sharing", keywords:"localsend kde connect phone ssh remote login" },
 { page: "notifications", label:"Notifications", keywords:"history alerts banners notification" },
 { page: "mouse", label:"Mouse & touchpad", keywords:"focus natural scrolling pointer" },
 { page: "keyboard", tab:"layout", label:"Keyboard layout", keywords:"language xkb qwerty azerty dvorak", target:"Keyboard layout" },

	{"page": "appearance", "tab": "appearance", "label": "Gaps between windows", "target": "Gaps between windows", "keywords": "gaps between windows"},
	{"page": "appearance", "tab": "appearance", "label": "Gaps around the edge", "target": "Gaps around the edge", "keywords": "gaps around the edge"},
	{"page": "appearance", "tab": "appearance", "label": "Border width", "target": "Border width", "keywords": "border width"},
	{"page": "appearance", "tab": "appearance", "label": "Active border", "target": "Active border", "keywords": "active border"},
	{"page": "appearance", "tab": "appearance", "label": "Inactive border", "target": "Inactive border", "keywords": "inactive border"},
	{"page": "appearance", "tab": "appearance", "label": "Layout", "target": "Layout", "keywords": "layout"},
	{"page": "appearance", "tab": "appearance", "label": "Resize by dragging the border", "target": "Resize by dragging the border", "keywords": "resize by dragging the border"},
	{"page": "appearance", "tab": "appearance", "label": "Corner rounding", "target": "Corner rounding", "keywords": "corner rounding"},
	{"page": "appearance", "tab": "appearance", "label": "Blur", "target": "Blur", "keywords": "blur"},
	{"page": "appearance", "tab": "appearance", "label": "Blur size", "target": "Blur size", "keywords": "blur size"},
	{"page": "appearance", "tab": "appearance", "label": "Blur quality", "target": "Blur quality", "keywords": "blur quality"},
	{"page": "appearance", "tab": "appearance", "label": "Shadows", "target": "Shadows", "keywords": "shadows"},
	{"page": "appearance", "tab": "appearance", "label": "Shadow size", "target": "Shadow size", "keywords": "shadow size"},
	{"page": "keyboard", "tab": "layout", "label": "Keyboard layout", "target": "Keyboard layout", "keywords": "keyboard layout"},
	{"page": "mouse", "label": "Focus", "target": "Focus", "keywords": "focus"},
	{"page": "mouse", "label": "Natural scrolling", "target": "Natural scrolling", "keywords": "natural scrolling"},
	{"page": "keyboard", "tab": "keybinds", "label": "Default mods", "target": "Default mods", "keywords": "default mods"},
	{"page": "keyboard", "tab": "keybinds", "label": "Active layout", "target": "Active layout", "keywords": "active layout"},
	{"page": "displays", "tab": "nightlight", "label": "Night light", "target": "Night light", "keywords": "night light"},
	{"page": "displays", "tab": "nightlight", "label": "When", "target": "When", "keywords": "when"},
	{"page": "displays", "tab": "nightlight", "label": "From", "target": "From", "keywords": "from"},
	{"page": "displays", "tab": "nightlight", "label": "To", "target": "To", "keywords": "to"},
	{"page": "displays", "tab": "nightlight", "label": "Warmth", "target": "Warmth", "keywords": "warmth"},
	{"page": "displays", "tab": "hyprland", "label": "Resolution", "target": "Resolution", "keywords": "resolution"},
	{"page": "displays", "tab": "hyprland", "label": "Scale", "target": "Scale", "keywords": "scale"},
	{"page": "displays", "tab": "hyprland", "label": "Rotation", "target": "Rotation", "keywords": "rotation"},
	{"page": "displays", "tab": "hyprland", "label": "Adaptive sync", "target": "Adaptive sync", "keywords": "adaptive sync"},
	{"page": "displays", "tab": "hyprland", "label": "Position", "target": "Position", "keywords": "position"},
	{"page": "power", "label": "Screen brightness", "target": "Screen brightness", "keywords": "screen brightness"},
	{"page": "power", "label": "Protect battery", "target": "Protect battery", "keywords": "protect battery"},
	{"page": "power", "label": "Start charging below", "target": "Start charging below", "keywords": "start charging below"},
	{"page": "power", "label": "Stop charging at", "target": "Stop charging at", "keywords": "stop charging at"},
	{"page": "sound", "label": "Volume", "target": "Volume", "keywords": "volume"},
	{"page": "sound", "label": "Mute", "target": "Mute", "keywords": "mute"},
	{"page": "desktop", "tab": "breadbar", "label": "Font", "target": "Font", "keywords": "font"},
	{"page": "desktop", "tab": "breadbar", "label": "Font size", "target": "Font size", "keywords": "font size"},
	{"page": "desktop", "tab": "breadbar", "label": "Corner rounding", "target": "Corner rounding", "keywords": "corner rounding"},
	{"page": "desktop", "tab": "breadbar", "label": "Inner padding", "target": "Inner padding", "keywords": "inner padding"},
	{"page": "desktop", "tab": "breadbar", "label": "Number size", "target": "Number size", "keywords": "number size"},
	{"page": "desktop", "tab": "breadbar", "label": "Inactive dimness", "target": "Inactive dimness", "keywords": "inactive dimness"},
	{"page": "desktop", "tab": "breadbar", "label": "Gap between stats", "target": "Gap between stats", "keywords": "gap between stats"},
	{"page": "desktop", "tab": "breadbar", "label": "Tray icon size", "target": "Tray icon size", "keywords": "tray icon size"},
	{"page": "desktop", "tab": "breadbar", "label": "Notification rounding", "target": "Notification rounding", "keywords": "notification rounding"},
	{"page": "lock", "label": "Background", "target": "Background", "keywords": "background"},
	{"page": "lock", "label": "Image", "target": "Image", "keywords": "image"},
	{"page": "lock", "label": "Blur background", "target": "Blur background", "keywords": "blur background"},
	{"page": "lock", "label": "Clock format", "target": "Clock format", "keywords": "clock format"},
	{"page": "lock", "label": "Font", "target": "Font", "keywords": "font"},
	{"page": "lock", "label": "Wrong-password timeout (ms)", "target": "Wrong-password timeout (ms)", "keywords": "wrong-password timeout (ms)"},
	{"page": "desktop", "tab": "breadshot", "label": "Save directory", "target": "Save directory", "keywords": "save directory"},
	{"page": "desktop", "tab": "breadshot", "label": "Silent (no notifications)", "target": "Silent (no notifications)", "keywords": "silent (no notifications)"},
	{"page": "desktop", "tab": "breadshot", "label": "Freeze screen during select", "target": "Freeze screen during select", "keywords": "freeze screen during select"},
	{"page": "desktop", "tab": "breadshot", "label": "Notification timeout (ms)", "target": "Notification timeout (ms)", "keywords": "notification timeout (ms)"},
	{"page": "desktop", "tab": "breadshot", "label": "Filename date format", "target": "Filename date format", "keywords": "filename date format"},
	{"page": "desktop", "tab": "breadpad", "label": "Default note type", "target": "Default note type", "keywords": "default note type"},
	{"page": "desktop", "tab": "breadpad", "label": "Tag with active workspace", "target": "Tag with active workspace", "keywords": "tag with active workspace"},
	{"page": "desktop", "tab": "breadpad", "label": "Snooze options", "target": "Snooze options", "keywords": "snooze options"},
	{"page": "desktop", "tab": "breadpad", "label": "Archive after (days)", "target": "Archive after (days)", "keywords": "archive after (days)"},
	{"page": "desktop", "tab": "breadpad", "label": "ONNX model", "target": "ONNX model", "keywords": "onnx model"},
	{"page": "desktop", "tab": "breadpad", "label": "Tokenizer", "target": "Tokenizer", "keywords": "tokenizer"},
	{"page": "desktop", "tab": "breadpad", "label": "Use Ollama", "target": "Use Ollama", "keywords": "use ollama"},
	{"page": "desktop", "tab": "breadpad", "label": "Endpoint", "target": "Endpoint", "keywords": "endpoint"},
	{"page": "desktop", "tab": "breadpad", "label": "Model", "target": "Model", "keywords": "model"},
	{"page": "desktop", "tab": "breadpad", "label": "Confidence threshold", "target": "Confidence threshold", "keywords": "confidence threshold"},
	{"page": "desktop", "tab": "breadpad", "label": "Default morning time", "target": "Default morning time", "keywords": "default morning time"},
	{"page": "desktop", "tab": "breadpad", "label": "Missed grace (minutes)", "target": "Missed grace (minutes)", "keywords": "missed grace (minutes)"},
	{"page": "desktop", "tab": "breadpad", "label": "Sync to calendar", "target": "Sync to calendar", "keywords": "sync to calendar"},
	{"page": "desktop", "tab": "breadpad", "label": "CalDAV URL", "target": "CalDAV URL", "keywords": "caldav url"},
	{"page": "desktop", "tab": "breadpad", "label": "Username", "target": "Username", "keywords": "username"},
	{"page": "desktop", "tab": "breadpad", "label": "Password", "target": "Password", "keywords": "password"},
	{"page": "desktop", "tab": "breadsearch", "label": "Enabled", "target": "Enabled", "keywords": "enabled"},
	{"page": "desktop", "tab": "breadsearch", "label": "Index while on battery", "target": "Index while on battery", "keywords": "index while on battery"},
	{"page": "desktop", "tab": "breadsearch", "label": "Search acceleration", "target": "Search acceleration", "keywords": "search acceleration"},
	{"page": "desktop", "tab": "breadsearch", "label": "Max file size (MB)", "target": "Max file size (MB)", "keywords": "max file size (mb)"},
	{"page": "desktop", "tab": "breadsearch", "label": "Result limit", "target": "Result limit", "keywords": "result limit"},
	{"page": "desktop", "tab": "breadsearch", "label": "Snippet length", "target": "Snippet length", "keywords": "snippet length"},
	{"page": "desktop", "tab": "breadsearch", "label": "Folders to index", "target": "Folders to index", "keywords": "folders to index"},
	{"page": "desktop", "tab": "breadsearch", "label": "Excluded folders", "target": "Excluded folders", "keywords": "excluded folders"},
	{"page": "desktop", "tab": "breadsearch", "label": "File extensions", "target": "File extensions", "keywords": "file extensions"},
	{"page": "desktop", "tab": "bread", "label": "Log level", "target": "Log level", "keywords": "log level"},
	{"page": "desktop", "tab": "bread", "label": "Socket path", "target": "Socket path", "keywords": "socket path"},
	{"page": "desktop", "tab": "bread", "label": "Entry point", "target": "Entry point", "keywords": "entry point"},
	{"page": "desktop", "tab": "bread", "label": "Module path", "target": "Module path", "keywords": "module path"},
	{"page": "desktop", "tab": "bread", "label": "Load built-in modules", "target": "Load built-in modules", "keywords": "load built-in modules"},
	{"page": "desktop", "tab": "bread", "label": "Disabled modules", "target": "Disabled modules", "keywords": "disabled modules"},
	{"page": "desktop", "tab": "bread", "label": "Hyprland", "target": "Hyprland", "keywords": "hyprland"},
	{"page": "desktop", "tab": "bread", "label": "udev (devices)", "target": "udev (devices)", "keywords": "udev (devices)"},
	{"page": "desktop", "tab": "bread", "label": "udev subsystems", "target": "udev subsystems", "keywords": "udev subsystems"},
	{"page": "desktop", "tab": "bread", "label": "Power", "target": "Power", "keywords": "power"},
	{"page": "desktop", "tab": "bread", "label": "Power poll interval (s)", "target": "Power poll interval (s)", "keywords": "power poll interval (s)"},
	{"page": "desktop", "tab": "bread", "label": "Network", "target": "Network", "keywords": "network"},
	{"page": "desktop", "tab": "bread", "label": "Bluetooth", "target": "Bluetooth", "keywords": "bluetooth"},
	{"page": "desktop", "tab": "bread", "label": "Dedup window (ms)", "target": "Dedup window (ms)", "keywords": "dedup window (ms)"},
	{"page": "desktop", "tab": "bread", "label": "Default timeout (ms)", "target": "Default timeout (ms)", "keywords": "default timeout (ms)"},
	{"page": "desktop", "tab": "bread", "label": "Default urgency", "target": "Default urgency", "keywords": "default urgency"},
	{"page": "desktop", "tab": "bread", "label": "notify-send path", "target": "notify-send path", "keywords": "notify-send path"},
	{"page": "restore", "tab": "backup", "label": "Local path", "target": "Local path", "keywords": "local path"},
	{"page": "restore", "tab": "backup", "label": "Or SFTP", "target": "Or SFTP", "keywords": "or sftp"},
	{"page": "restore", "tab": "backup", "label": "Restore into", "target": "Restore into", "keywords": "restore into"},
	{"page": "network", "tab": "breadcrumbs", "label": "Places", "target": "Places", "keywords": "places profiles location home school work breadcrumbs switch automatically"},
	{"page": "network", "tab": "advanced", "label": "DNS", "target": "DNS", "keywords": "dns cloudflare quad9 google servers"},
	{"page": "network", "tab": "advanced", "label": "Tailscale exit node", "target": "Tailscale exit node", "keywords": "exit node tailscale"},
	{"page": "network", "tab": "advanced", "label": "Sign-in page check", "target": "Sign-in page check", "keywords": "captive portal connectivity check url"},
	{"page": "network", "tab": "advanced", "label": "Reachability check", "target": "Reachability check", "keywords": "ping host"},
	{"page": "network", "tab": "advanced", "label": "Give up joining a network after", "target": "Give up joining a network after", "keywords": "connection timeout wait"},
	{"page": "network", "tab": "advanced", "label": "Check the connection every", "target": "Check the connection every", "keywords": "connectivity interval watch"},
	{"page": "network", "tab": "advanced", "label": "More connection settings", "target": "More connection settings", "keywords": "connection editor ip proxy static ethernet"},
	{"page": "a11y", "label": "Zoom", "target": "Zoom", "keywords": "zoom magnifier magnify enlarge"},

	{ page: "home", label: "Home", keywords: "search overview status" },
	{
		page: "network",
		label: "Wi-Fi",
		keywords: "wifi ssid password scan radio ethernet hotspot dns",
	},
	{
		page: "network",
		tab: "vpn",
		label: "VPN",
		keywords: "vpn wireguard openvpn tunnel import conf ovpn other connections",
	},
	{
		page: "network",
		label: "Saved networks",
		keywords: "wifi known ssid forget password hidden network",
	},
	{ page: "bluetooth", label: "Bluetooth", keywords: "bluetooth headphones pair scan mouse keyboard" },
	{
		page: "displays",
		label: "Displays",
		keywords: "monitor scale resolution arrange rotate vrr hyprland breadmon",
	},
	{
		page: "displays",
		tab: "nightlight",
		label: "Night light",
		keywords: "night light hyprsunset temperature warmth sunset",
	},
	{
		page: "appearance",
		label: "Wallpaper",
		keywords: "wallpaper theme palette colours breadpaper accent per monitor multi monitor background",
	},
	{
		page: "appearance",
		tab: "shelltheme",
		label: "Shell theme",
		keywords: "shell theme breadbar breadbox bar launcher style liquid motion glass workbench spotlight daylight layout animation",
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
		label: "Keyboard",
		keywords: "keybind shortcut layout shortcut keys",
	},
	{ page: "input", tab: "ime", label: "Input method", keywords: "fcitx5 ime cjk chinese japanese korean" },
	{
		page: "input",
		tab: "accessibility",
		label: "Accessibility",
		keywords: "orca screen reader zoom magnifier orca",
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


function editDistance(a: string, b: string): number {
 let row = Array.from({ length: b.length + 1 }, (_, i) => i);
 for (let i = 1; i <= a.length; i++) {
  const next = [i];
  for (let j = 1; j <= b.length; j++) next[j] = Math.min(next[j - 1] + 1, row[j] + 1, row[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1));
  row = next;
 }
 return row[b.length];
}

export function searchSettings(query: string, limit = 8): SearchHit[] {
 const words = query.toLowerCase().trim().split(/\s+/).filter(Boolean);
 const pages: SearchHit[] = SIDEBAR_SECTIONS.flatMap(section => section.items.map(item => ({page:item.id,label:item.label,keywords:item.label.toLowerCase()})));
 const indexed = [...pages, ...SEARCH_INDEX].map((hit) => {
  const resolved = resolveDestination(hit.page, hit.tab);
  const page = resolved.page;
  const tab = resolved.tab;
  const label = SIDEBAR_SECTIONS.flatMap(s => s.items).find(i => i.id === page)?.label ?? page;
  return { ...hit, page, tab, context: hit.context ?? `${label}${tab ? " › " + tab : ""}` };
 });
 const unique = [...new Map(indexed.map(hit => [`${hit.page}:${hit.tab ?? ""}:${hit.label}`, hit])).values()];
 return unique.map(hit => {
  const label = hit.label.toLowerCase();
  const tokens = `${label} ${hit.keywords}`.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  const scores = words.map(word => {
   if (label === word) return 20;
   if (label.startsWith(word)) return 12;
   if (tokens.some(t => t === word)) return 10;
   if (tokens.some(t => t.startsWith(word))) return 6;
   if (word.length >= 4 && tokens.some(t => editDistance(t, word) <= (word.length > 7 ? 2 : 1))) return 3;
   return 0;
  });
  return { hit, score: words.length ? (scores.every(s => s > 0) ? scores.reduce<number>((a,b) => a+b, 0) : 0) : 1 };
 }).filter(x => x.score > 0).sort((a,b) => b.score-a.score).slice(0,limit).map(x => x.hit);
}
