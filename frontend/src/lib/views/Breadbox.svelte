<script lang="ts">
	import { useDraft } from "$lib/drafts.svelte";
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import Group from "$lib/components/Group.svelte";
	import Sheet from "$lib/components/Sheet.svelte";

	interface Context {
		name: string;
		priority: string[];
	}

	let contexts = $state<Context[] | null>(null);
	let editing = $state<Context | null>(null);

	onMount(async () => {
		contexts = await invoke<Context[]>("get_breadbox_contexts");
	});

	function addContext() {
		let name = "work";
		for (let n = 2; contexts?.some((c) => c.name === name); n++) name = `work ${n}`;
		contexts = [...(contexts ?? []), { name, priority: [] }];
	}

	function removeContext(i: number) {
		contexts = contexts!.filter((_, idx) => idx !== i);
	}

	function priorityText(ctx: Context) {
		return ctx.priority.join(", ");
	}

	function setPriority(ctx: Context, text: string) {
		ctx.priority = text
			.split(",")
			.map((s) => s.trim())
			.filter((s) => s.length > 0);
	}

	async function save() {
		await invoke("save_breadbox_contexts", { contexts });
	}
	useDraft(() => (contexts), save, () => contexts !== null);
</script>

<ViewScaffold title="Launcher">
	{#if contexts}
		<Group title="Launcher" hint="Apps and categories the launcher lists first.">
			{#each contexts as ctx, i (i)}
				<div class="ui-item">
					<div class="lb"><b>{ctx.name === "default" ? "Shown first" : `Shown first in ${ctx.name}`}</b><small>{ctx.priority.length ? priorityText(ctx) : "Nothing yet"}</small></div>
					<button class="btn small" onclick={() => (editing = ctx)}>Edit</button>
				</div>
			{/each}
			{#if contexts.length === 0}
				<div class="ui-item">
					<div class="lb"><b>Shown first</b><small>Nothing yet</small></div>
					<button class="btn small" onclick={() => { contexts = [{ name: "default", priority: [] }]; editing = contexts[0]; }}>Edit</button>
				</div>
			{/if}
			<details class="ui-disclosure">
				<summary>More</summary>
				<div class="ui-item">
					<div class="lb"><b>Another list</b><small>A separate list for a launcher context, if your setup switches between them.</small></div>
					<button class="btn small ghost" onclick={() => { addContext(); editing = contexts![contexts!.length - 1]; }}>Add</button>
				</div>
				<ServiceControl unit="breadbox-sync.service" name="Launcher app list sync" bare hasConfig />
			</details>
		</Group>
	{/if}
</ViewScaffold>

{#if editing}
	{@const ctx = editing}
	<Sheet title={ctx.name === "default" ? "Shown first" : `Shown first in ${ctx.name}`} lede="App names or categories, in order, separated by commas." onClose={() => (editing = null)}>
		{#if ctx.name !== "default"}<label class="field"><span>Context name</span><input class="ui-input" bind:value={ctx.name} /></label>{/if}
		<label class="field"><span>Apps and categories</span><input class="ui-input" value={priorityText(ctx)} onchange={(e) => setPriority(ctx, e.currentTarget.value)} placeholder="Zen Browser, kitty, Development" /></label>
		{#snippet actions()}
			{#if ctx.name !== "default"}<button class="btn remove" onclick={() => { removeContext(contexts!.indexOf(ctx)); editing = null; }}>Remove</button><span class="spacer"></span>{/if}
			<button class="btn primary" onclick={() => (editing = null)}>Done</button>
		{/snippet}
	</Sheet>
{/if}

<style>
	.field { display:flex; flex-direction:column; gap:6px; padding:8px 0; font-size:13px; }
	.spacer { flex:1; }
</style>
