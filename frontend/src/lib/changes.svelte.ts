export interface RecentChange { id:number; label:string; at:Date; undo:(() => Promise<void>)|null }
export const changes = $state({ recent: [] as RecentChange[], message:"", undo: null as (() => Promise<void>)|null, busy:false });
let nextId = 0;
export async function recordChange(label:string, apply:() => Promise<void>, undo:() => Promise<void>) {
 await apply();
 const change: RecentChange = { id:++nextId, label, at:new Date(), undo };
 changes.recent = [change, ...changes.recent].slice(0,6);
 const entry = changes.recent[0];
 changes.message = label;
 changes.undo = () => undoChange(entry);
}
export async function undoChange(entry: RecentChange) {
 if(!entry.undo || changes.busy) return;
 changes.busy = true;
 try { await entry.undo(); entry.undo = null; changes.undo = null; changes.message = "Change undone"; }
 catch(e) { changes.message = `Could not undo: ${String(e)}`; }
 finally { changes.busy = false; }
}
