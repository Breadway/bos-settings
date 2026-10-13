import assert from "node:assert/strict";
import { createServer, DevEnvironment, createRunnableDevEnvironment } from "vite";

// Load Svelte/TypeScript modules without serving or launching the UI.
const server = await createServer({
 server: { middlewareMode: true, hmr: false, watch: null }, appType: "custom",
 environments: {
  client: { dev: { createEnvironment: (name, config) => new DevEnvironment(name, config, { hot:false }) } },
  ssr: { dev: { createEnvironment: (name, config) => createRunnableDevEnvironment(name, config, { hot:false }) } },
 },
});
try {
 const { searchSettings } = await server.ssrLoadModule("/src/lib/search.ts");
 const { SIDEBAR_SECTIONS, PAGE_ALIASES, resolveDestination } = await server.ssrLoadModule("/src/lib/sidebar.ts");
 const { VIEWS } = await server.ssrLoadModule("/src/lib/views/registry.ts");
 const { drafts, guardNavigation, saveDrafts, discardAndContinue } = await server.ssrLoadModule("/src/lib/drafts.svelte.ts");
 const { isSystemStartup } = await server.ssrLoadModule("/src/lib/systemStartup.ts");
 for (const item of SIDEBAR_SECTIONS.flatMap(s => s.items)) assert.ok(VIEWS[item.id], `Missing view: ${item.id}`);
 for (const alias of Object.keys(PAGE_ALIASES)) assert.ok(VIEWS[resolveDestination(alias).page], `Broken alias: ${alias}`);
 assert.deepEqual(resolveDestination("desktop", "breadlock"), {page:"lock"});
 assert.deepEqual(resolveDestination("system", "backup"), {page:"restore",tab:"backup"});
 assert.deepEqual(resolveDestination("input", "accessibility"), {page:"a11y"});
 assert.ok(searchSettings("wallpapr").some(h => h.label === "Wallpaper"), "A missing letter should find Wallpaper");
 assert.ok(searchSettings("natural scrolling").some(h => h.page === "mouse" && h.target === "Natural scrolling"));
 assert.deepEqual(searchSettings("nothingmatchesxyz"), []);
 assert.deepEqual(searchSettings("hdr"), [], "Do not advertise unsupported HDR controls");
 for (const query of ["layout", "keyboard", "night", "", "updates"]) {
  const hits = searchSettings(query, 200);
  const keys = hits.map(h => `${h.page}:${h.tab ?? ""}:${h.label}`);
  assert.equal(new Set(keys).size,keys.length,`Duplicate keyed result for ${query}`);
 }
 assert.ok(isSystemStartup("breadbar"));
 assert.ok(isSystemStartup("bash -c 'command -v breadshot >/dev/null && exec breadshot listen'"));
 assert.ok(!isSystemStartup("breadbar --my-custom-bar"));
 let navigated = 0;
 drafts.items = [{dirty:true,save:async()=>{throw new Error("save refused");}}];
 guardNavigation(() => navigated++);
 assert.equal(navigated,0,"Dirty navigation must be held");
 await saveDrafts();
 assert.equal(navigated,0,"A failed save must not navigate");
 assert.ok(drafts.error.includes("save refused"));
 assert.ok(drafts.pending,"Keep the destination after save failure");
 drafts.items[0].save = async () => {drafts.items[0].dirty = false;};
 await saveDrafts();
 assert.equal(navigated,1,"Navigate after a successful save");
 drafts.items[0].dirty = true;
 drafts.items[0].save = async () => {throw new Error("still refused");};
 guardNavigation(() => navigated++);
 await saveDrafts();
 discardAndContinue();
 assert.equal(navigated,2,"Explicit discard allows navigation");
 drafts.items = [];
 console.log("v2: navigation aliases, sidebar coverage, search destinations, fuzzy matching, startup classification, and save/discard guards passed");
} finally { await server.close(); }
