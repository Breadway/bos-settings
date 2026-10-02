/** Keep the last successful readings when Home is unmounted. */
export const home = $state({
 wifi:null as {radio_enabled:boolean; ethernet:string|null}|null,
 bluetooth:null as boolean|null,
 night:null as Record<string,unknown>|null,
 power:null as {battery:[string,string][];power_source:string}|null,
 updates:null as {pacman:unknown[]; bakery:unknown[]; firmware:unknown[]; pacman_error:string|null; bakery_error:string|null}|null,
 backup:null as {repo:string; error:string|null}|null,
 lastCheck:0,
 errors:{} as Record<string,string>,
});
