<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import ServiceControl from "$lib/components/ServiceControl.svelte";
	import Group from "$lib/components/Group.svelte";
	import SaveButton from "$lib/components/SaveButton.svelte";

	interface Context {
		name: string;
		priority: string[];
	}

	let contexts = $state<Context[] | null>(null);

	onMount(async () => {
		contexts = await invoke<Context[]>("get_breadbox_contexts");
	});

	function addContext() {
		contexts = [...(contexts ?? []), { name: "new", priority: [] }];
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
</script>

<ViewScaffold title="Launcher">
	{#if contexts}
		<Group
			title="Contexts"
			hint="Apps and categories shown first."
			wide
		>
			{#if contexts.length === 0}
				<div class="empty">No launcher contexts yet. Add one to control which apps/categories breadbox surfaces first.</div>
			{/if}
			{#each contexts as ctx, i (i)}
				<div class="row">
					<input type="text" bind:value={ctx.name} placeholder="name" class="name" />
					<input
						type="text"
						value={priorityText(ctx)}
						oninput={(e) => setPriority(ctx, e.currentTarget.value)}
						placeholder="firefox, code, Development, ..."
						class="priority"
					/>
					<button class="remove" onclick={() => removeContext(i)}>Remove</button>
				</div>
			{/each}
			<button class="add" onclick={addContext}>Add context</button>
		</Group>

		<SaveButton onSave={save} />
	{/if}

	<ServiceControl unit="breadbox-sync.service" hasConfig />
</ViewScaffold>

<style>
	.row {
		display: flex;
		gap: var(--space-sm, 8px);
		align-items: center;
		margin-bottom: var(--space-xs, 4px);
	}

	.name {
		width: 14ch;
	}

	.priority {
		flex: 1;
	}

	input {
		background-color: var(--surface);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: var(--radius-secondary, 6px);
		padding: var(--space-xs, 4px) var(--space-sm, 8px);
	}

	input:focus {
		outline: none;
		border-color: var(--accent);
	}

	button {
		border: none;
		border-radius: var(--radius-primary, 8px);
		padding: var(--space-xs, 4px) var(--space-md, 12px);
		cursor: pointer;
	}

	.remove {
		background-color: var(--red);
		color: var(--on-red);
	}

	.add {
		background-color: var(--surface);
		color: var(--on-surface);
		margin-top: var(--space-sm, 8px);
	}

	.empty {
		opacity: 0.6;
		font-size: var(--font-size-secondary, 12px);
		padding: var(--space-md, 12px) 0;
	}
</style>
