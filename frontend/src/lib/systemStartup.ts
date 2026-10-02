/** Stock desktop infrastructure, matched exactly so a user command with a similar name stays editable. */
const CORE = new Set(["breadbar", "hypridle", "bos-netcheck", "breadhelp --autostart", "bash -c 'command -v bos-first-boot >/dev/null && exec bos-first-boot'"]);
const BUSES = ["breadpaper", "breadshot", "breadlock", "breadbox", "breadhelp", "breadpad", "breadclip", "breadsearch", "bread"];
export function isSystemStartup(command: string): boolean {
 return CORE.has(command) || BUSES.some(cmd => command === `bash -c 'command -v ${cmd} >/dev/null && exec ${cmd} listen'`) || /^hyprsunset(?: -t \d+)?$/.test(command);
}
