<script lang="ts">
	import Search from "@lucide/svelte/icons/search";
	import { SIDEBAR_SECTIONS } from "$lib/sidebar";
	import { go, nav } from "$lib/nav.svelte";
	let { openSearch }: { openSearch: () => void } = $props();
</script>

<nav class="sidebar" aria-label="Settings categories">
	<button class="search" onclick={openSearch} title="Search settings (Ctrl K)" aria-label="Search settings">
		<Search size={17} /><span>Search settings</span><kbd>Ctrl K</kbd>
	</button>
	<div class="nav">
		{#each SIDEBAR_SECTIONS as section (section.title ?? "home")}
			{#if section.title}<div class="section-header">{section.title}</div>{/if}
			{#each section.items as item (item.id)}
				<button class="item" class:selected={nav.page === item.id} aria-current={nav.page === item.id ? "page" : undefined}
					aria-label={item.label} title={item.label} onclick={() => go(item.id)}>
					<item.icon size={17} /><span>{item.label}</span>
				</button>
			{/each}
		{/each}
	</div>
	<div class="brand"><div class="avatar">B</div><div><b>BOS Settings</b><small>Bread OS</small></div></div>
</nav>

<style>
	.sidebar { width:236px; flex:none; display:flex; flex-direction:column; min-height:0; background:var(--bg-2); border-right:1px solid var(--line); }
	.search { margin:14px 12px 8px; display:flex; align-items:center; gap:9px; padding:9px 11px; border-radius:10px; background:var(--surface-2); color:var(--muted); border:1px solid var(--line); text-align:left; cursor:pointer; font-size:13px; }
	.search kbd { margin-left:auto; white-space:nowrap; font-size:10px; }
	.nav { flex:1; overflow-y:auto; padding:2px 10px 12px; scrollbar-width:thin; }
	.section-header { font-size:10px; font-weight:500; letter-spacing:.12em; text-transform:uppercase; color:var(--faint); padding:14px 10px 6px; }
	.item { display:flex; align-items:center; gap:11px; width:100%; padding:7px 10px; border:0; border-radius:9px; color:var(--fg); background:transparent; text-align:left; font-size:13px; cursor:pointer; }
	.item:hover { background:color-mix(in srgb,var(--fg) 4%,transparent); }
	.item.selected { background:var(--accent-soft); }
	.item.selected :global(svg) { color:var(--accent); }
	.item :global(svg), .search :global(svg) { flex:none; }
	.brand { display:flex; align-items:center; gap:10px; padding:12px 14px; border-top:1px solid var(--line); font-size:12.5px; }
	.brand b { font-weight:400; display:block; } .brand small { color:var(--muted); }
	.avatar { width:28px; height:28px; border-radius:50%; display:grid; place-items:center; background:var(--accent-soft); color:var(--accent); }
	@media(max-width:900px) { .sidebar { width:64px; } .item { justify-content:center; padding:11px; } .item span,.section-header,.search span,.search kbd,.brand { display:none; } .search { margin:12px 8px 8px; justify-content:center; padding:10px; } .nav { padding:2px 6px 12px; } }
	@media(max-width:560px) { .sidebar { width:100%; flex-direction:row; height:56px; border-right:0; border-bottom:1px solid var(--line); } .search { margin:8px; flex:none; } .nav { display:flex; overflow-x:auto; overflow-y:hidden; padding:6px 6px 6px 0; gap:2px; } .item { width:40px; flex:none; padding:10px; } }
</style>
