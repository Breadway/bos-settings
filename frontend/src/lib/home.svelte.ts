/** Keep the last successful readings when Home is unmounted. */
import type {NotificationPreferences} from "./notificationPreferences";
export interface RadioState {id:number;kind:string;blocked:boolean;hardware_blocked:boolean}
export const home = $state({
 account:null as {username:string;display_name:string}|null,
 notifications:null as NotificationPreferences|null,
 modes:null as {backend:string;current:string;profiles:string[];tlp_setup_available?:boolean}|null,
 radios:null as RadioState[]|null,
 storage:null as {disks:{path:string;total:number;used:number;available:number}[];package_cache:number|null;app_cache:number|null;trash:number|null}|null,
 devices:null as {address:string;name:string;connected:boolean;battery:number|null}[]|null,
 automatic:false,
 wifi:null as {radio_enabled:boolean; ethernet:string|null;ssid:string|null}|null,
 bluetooth:null as boolean|null,
 night:null as Record<string,unknown>|null,
 power:null as {battery:[string,string][];power_source:string}|null,
 updates:null as {total:number; pacman:unknown[]; bakery:unknown[]; firmware:unknown[]; pacman_error:string|null; bakery_error:string|null}|null,
 backup:null as {repo:string; error:string|null}|null,
 lastCheck:0,
 errors:{} as Record<string,string>,
});
