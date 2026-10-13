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
	{"page": "keyboard", "tab": "keybinds", "label": "Main modifier key", "target": "Main modifier key", "keywords": "main modifier key default mods default mods"},
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
	{"page": "power", "label": "Brightness", "target": "Brightness", "keywords": "brightness screen brightness screen brightness"},
	{"page": "power", "label": "Protect battery", "target": "Protect battery", "keywords": "protect battery"},
	{"page": "power", "label": "Start charging below", "target": "Start charging below", "keywords": "start charging below"},
	{"page": "power", "label": "Stop charging at", "target": "Stop charging at", "keywords": "stop charging at"},
	{"page": "sound", "label": "Mute", "target": "Mute", "keywords": "mute"},
	{"page": "desktop", "tab": "breadbar", "label": "Font", "target": "Font", "keywords": "font"},
	{"page": "desktop", "tab": "breadbar", "label": "Text size", "target": "Text size", "keywords": "text size font size font size"},
	{"page": "desktop", "tab": "breadbar", "label": "Corner rounding", "target": "Corner rounding", "keywords": "corner rounding"},
	{"page": "desktop", "tab": "breadbar", "label": "Inner padding", "target": "Inner padding", "keywords": "inner padding"},
	{"page": "desktop", "tab": "breadbar", "label": "Workspace number size", "target": "Workspace number size", "keywords": "workspace number size number size number size"},
	{"page": "desktop", "tab": "breadbar", "label": "Inactive workspaces", "target": "Inactive workspaces", "keywords": "inactive workspaces inactive dimness inactive dimness"},
	{"page": "desktop", "tab": "breadbar", "label": "Gap between status items", "target": "Gap between status items", "keywords": "gap between status items gap between stats gap between stats"},
	{"page": "desktop", "tab": "breadbar", "label": "Tray icon size", "target": "Tray icon size", "keywords": "tray icon size"},
	{"page": "desktop", "tab": "breadbar", "label": "Notification corner rounding", "target": "Notification corner rounding", "keywords": "notification corner rounding notification rounding notification rounding"},
	{"page": "lock", "label": "Background", "target": "Background", "keywords": "background"},
	{"page": "lock", "label": "Picture", "target": "Picture", "keywords": "picture image image"},
	{"page": "lock", "label": "Clock format", "target": "Clock format", "keywords": "clock format"},
	{"page": "lock", "label": "Font", "target": "Font", "keywords": "font"},
	{"page": "lock", "label": "After a wrong password, wait", "target": "After a wrong password, wait", "keywords": "after a wrong password, wait wrong-password timeout (ms) wrong-password timeout (ms)"},
	{"page": "desktop", "tab": "breadshot", "label": "Save to", "target": "Save to", "keywords": "save to save directory save directory"},
	{"page": "desktop", "tab": "breadshot", "label": "Show a notification", "target": "Show a notification", "keywords": "show a notification silent (no notifications) silent (no notifications)"},
	{"page": "desktop", "tab": "breadshot", "label": "Freeze the screen while selecting", "target": "Freeze the screen while selecting", "keywords": "freeze the screen while selecting freeze screen during select freeze screen during select"},
	{"page": "desktop", "tab": "breadshot", "label": "Notification stays for", "target": "Notification stays for", "keywords": "notification stays for notification timeout (ms) notification timeout (ms)"},
	{"page": "desktop", "tab": "breadshot", "label": "File name", "target": "File name", "keywords": "file name filename date format filename date format"},
	{"page": "desktop", "tab": "breadpad", "label": "New notes start as", "target": "New notes start as", "keywords": "new notes start as default note type default note type"},
	{"page": "desktop", "tab": "breadpad", "label": "Remember the workspace", "target": "Remember the workspace", "keywords": "remember the workspace tag with active workspace tag with active workspace"},
	{"page": "desktop", "tab": "breadpad", "label": "Snooze choices", "target": "Snooze choices", "keywords": "snooze choices snooze options snooze options"},
	{"page": "desktop", "tab": "breadpad", "label": "Archive finished notes after", "target": "Archive finished notes after", "keywords": "archive finished notes after archive after (days) archive after (days)"},
	{"page": "desktop", "tab": "breadpad", "label": "Sorting model", "target": "Sorting model", "keywords": "sorting model onnx model onnx model"},
	{"page": "desktop", "tab": "breadpad", "label": "Model tokenizer", "target": "Model tokenizer", "keywords": "model tokenizer tokenizer tokenizer"},
	{"page": "desktop", "tab": "breadpad", "label": "Ask a local AI model (Ollama)", "target": "Ask a local AI model (Ollama)", "keywords": "ask a local ai model (ollama) use ollama use ollama"},
	{"page": "desktop", "tab": "breadpad", "label": "Ollama address", "target": "Ollama address", "keywords": "ollama address endpoint endpoint"},
	{"page": "desktop", "tab": "breadpad", "label": "Ollama model", "target": "Ollama model", "keywords": "ollama model model model"},
	{"page": "desktop", "tab": "breadpad", "label": "Only trust it above", "target": "Only trust it above", "keywords": "only trust it above confidence threshold confidence threshold"},
	{"page": "desktop", "tab": "breadpad", "label": "Morning reminders at", "target": "Morning reminders at", "keywords": "morning reminders at default morning time default morning time"},
	{"page": "desktop", "tab": "breadpad", "label": "Still show a missed reminder for (minutes)", "target": "Still show a missed reminder for (minutes)", "keywords": "still show a missed reminder for (minutes) missed grace (minutes) missed grace (minutes)"},
	{"page": "desktop", "tab": "breadpad", "label": "Add reminders to my calendar", "target": "Add reminders to my calendar", "keywords": "add reminders to my calendar sync to calendar sync to calendar"},
	{"page": "desktop", "tab": "breadpad", "label": "Calendar address", "target": "Calendar address", "keywords": "calendar address caldav url caldav url"},
	{"page": "desktop", "tab": "breadpad", "label": "Username", "target": "Username", "keywords": "username"},
	{"page": "desktop", "tab": "breadpad", "label": "Password", "target": "Password", "keywords": "password"},
	{"page": "desktop", "tab": "breadsearch", "label": "Add new and changed files", "target": "Add new and changed files", "keywords": "add new and changed files enabled enabled"},
	{"page": "desktop", "tab": "breadsearch", "label": "Keep going on battery", "target": "Keep going on battery", "keywords": "keep going on battery index while on battery index while on battery"},
	{"page": "desktop", "tab": "breadsearch", "label": "Speed up with", "target": "Speed up with", "keywords": "speed up with search acceleration search acceleration"},
	{"page": "desktop", "tab": "breadsearch", "label": "Skip files bigger than (MB)", "target": "Skip files bigger than (MB)", "keywords": "skip files bigger than (mb) max file size (mb) max file size (mb)"},
	{"page": "desktop", "tab": "breadsearch", "label": "Results to show", "target": "Results to show", "keywords": "results to show result limit result limit"},
	{"page": "desktop", "tab": "breadsearch", "label": "Preview length (characters)", "target": "Preview length (characters)", "keywords": "preview length (characters) snippet length snippet length"},
	{"page": "desktop", "tab": "breadsearch", "label": "Folders to search", "target": "Folders to search", "keywords": "folders to search folders to index folders to index"},
	{"page": "desktop", "tab": "breadsearch", "label": "Skip these folders", "target": "Skip these folders", "keywords": "skip these folders excluded folders excluded folders"},
	{"page": "desktop", "tab": "breadsearch", "label": "File types", "target": "File types", "keywords": "file types file extensions file extensions"},
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
	{"page": "restore", "tab": "backup", "label": "Folder", "target": "Folder", "keywords": "backup folder drive usb local path"},
	{"page": "restore", "tab": "backup", "label": "Back up to", "target": "Back up to", "keywords": "backup destination another computer ssh sftp server"},
	{"page": "restore", "tab": "backup", "label": "How often", "target": "How often", "keywords": "backup schedule daily weekly automatic"},
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
