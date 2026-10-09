<script lang="ts">
	import {useDraft} from "$lib/drafts.svelte";
    import {recordChange} from "$lib/changes.svelte";
    import { onMount, onDestroy } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import SelectField from "$lib/components/SelectField.svelte";
	import SliderField from "$lib/components/SliderField.svelte";
	import HyprColorField from "$lib/components/HyprColorField.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import { debounce } from "$lib/debounce";

	// Common XKB layout codes. Not exhaustive  -  if the config holds
	// something else, it's merged in below so it's never dropped from the
	// dropdown.
	const COMMON_KB_LAYOUTS = [
		"us", "gb", "de", "fr", "es", "it", "pt", "nl", "se", "no", "dk", "fi",
		"pl", "cz", "sk", "hu", "ro", "gr", "tr", "ru", "ua", "jp", "kr", "cn",
		"br", "ca", "ch", "be", "at", "ie",
	];

	// Hyprland's follow_mouse variable (see the wiki's Variables page):
	// 0 = cursor movement never changes focus; 1 = focus always follows the
	// window under the cursor; 2 = focus follows the cursor, but clicking a
	// window keeps keyboard focus there until the mouse moves again; 3 =
	// cursor and keyboard focus are fully independent.
	const FOLLOW_MOUSE_MODES: { value: string; label: string }[] = [
		{ value: "0", label: "Click to focus" },
		{ value: "1", label: "Follow mouse" },
		{ value: "2", label: "Follow, keep focus on click" },
		{ value: "3", label: "Mouse and keys independent" },
	];

	const KB_LABELS: Record<string, string> = {
		us: "English (US)",
		gb: "English (UK)",
		de: "German",
		fr: "French",
		es: "Spanish",
		it: "Italian",
		pt: "Portuguese",
		nl: "Dutch",
		se: "Swedish",
		no: "Norwegian",
		dk: "Danish",
		fi: "Finnish",
		pl: "Polish",
		cz: "Czech",
		sk: "Slovak",
		hu: "Hungarian",
		ro: "Romanian",
		gr: "Greek",
		tr: "Turkish",
		ru: "Russian",
		ua: "Ukrainian",
		jp: "Japanese",
		kr: "Korean",
		cn: "Chinese",
		br: "Portuguese (Brazil)",
		ca: "English (Canada)",
		ch: "German (Switzerland)",
		be: "Belgian",
		at: "German (Austria)",
		ie: "English (Ireland)",
	};

	const LAYOUT_LABELS: Record<string, string> = {
		dwindle: "Tiling",
		master: "Master stack",
	};

	interface Appearance {
		gaps_in: number;
		gaps_out: number;
		border_size: number;
		active_border: string;
		inactive_border: string;
		layout: string;
		resize_on_border: boolean;
		rounding: number;
		blur_enabled: boolean;
		blur_size: number;
		blur_passes: number;
		shadow_enabled: boolean;
		shadow_range: number;
		shadow_render_power: number;
		kb_layout: string;
		follow_mouse: number;
		natural_scroll: boolean;
	}

	let { scope = "all" }: { scope?: "all" | "windows" | "keyboard" | "mouse" } = $props();
	let cfg = $state<Appearance | null>(null);
	let loaded = $state(false);
	let followMouse = $state("1");

	let kbLayoutOptions = $derived(
		cfg && !COMMON_KB_LAYOUTS.includes(cfg.kb_layout) ? [...COMMON_KB_LAYOUTS, cfg.kb_layout] : COMMON_KB_LAYOUTS,
	);

    let mounted=true;onDestroy(()=>mounted=false);
    let status=$state("");let previous:Appearance|null=null;let queue=Promise.resolve();
    async function saveNow(){
        if(!mounted||!cfg)return;
        const next={...$state.snapshot(cfg),follow_mouse:Number(followMouse)};
        queue=queue.catch(()=>{}).then(async()=>{
            if(!mounted)return;
            const before=previous ?? await invoke<Appearance>("get_appearance");
            const keys=(Object.keys(next) as (keyof Appearance)[]).filter(k=>next[k]!==before[k]);
            if(!keys.length)return;
            const labels:Partial<Record<keyof Appearance,string>>={gaps_in:"Space between windows",gaps_out:"Space around the edge",rounding:"Corner rounding",border_size:"Border width",kb_layout:"Keyboard layout",follow_mouse:"Focus",natural_scroll:"Natural scrolling"};
            await recordChange(keys.length===1 ? `${labels[keys[0]] ?? keys[0].replaceAll('_',' ')} set to ${next[keys[0]]}` : "Window settings changed",()=>invoke<void>("save_appearance",{appearance:next}),async()=>{
                const latest=await invoke<Appearance>("get_appearance");
                const restored={...latest};for(const key of keys)Object.assign(restored,{[key]:before[key]});
                await invoke("save_appearance",{appearance:restored});previous=restored;cfg=restored;followMouse=String(restored.follow_mouse);
            });
            previous=next;draft.markSaved(JSON.stringify(next));status="";
        });
        return queue;
    }
    const draft=useDraft(()=>cfg ? {...cfg,follow_mouse:Number(followMouse)} : null,saveNow,()=>loaded);
    const persist=debounce(()=>{void saveNow().catch(e=>status=String(e));},450);

	onMount(async () => {
		cfg = await invoke<Appearance>("get_appearance");
		followMouse = String(cfg.follow_mouse);
        previous={...$state.snapshot(cfg)};
		loaded = true;
	});

	let primed = false;
	$effect(() => {
		if (!loaded || !cfg) return;
		JSON.stringify(cfg);
		void followMouse;
		if (!primed) {
			primed = true;
			return;
		}
		persist();
	});
</script>

<ViewScaffold title={scope === "mouse" ? "Mouse & touchpad" : scope === "keyboard" ? "Keyboard layout" : "Appearance"}>
	{#if cfg}
		{#if scope === "all" || scope === "windows"}
		<Group title="Windows">
            <div class="window-preview" style={`--gi:${cfg.gaps_in*0.6}px;--go:${cfg.gaps_out*0.6}px;--r:${cfg.rounding}px;--bw:${cfg.border_size}px`} aria-label="Window spacing preview"><i></i><i></i><i></i></div>
			<SliderField label="Gaps between windows" bind:value={cfg.gaps_in} min={0} max={50} />
			<SliderField label="Gaps around the edge" bind:value={cfg.gaps_out} min={0} max={50} />
			<SliderField label="Border width" bind:value={cfg.border_size} min={0} max={10} />
			<HyprColorField label="Active border" bind:value={cfg.active_border} />
			<HyprColorField label="Inactive border" bind:value={cfg.inactive_border} />
			<SelectField label="Layout" bind:value={cfg.layout} options={["dwindle", "master"]} labels={LAYOUT_LABELS} />
			<SwitchField label="Resize by dragging the border" bind:value={cfg.resize_on_border} />
		</Group>

		<Group title="Effects">
			<SliderField label="Corner rounding" bind:value={cfg.rounding} min={0} max={30} />
			<SwitchField label="Blur" bind:value={cfg.blur_enabled} />
			{#if cfg.blur_enabled}
				<SliderField label="Blur size" bind:value={cfg.blur_size} min={0} max={20} />
				<SliderField label="Blur quality" bind:value={cfg.blur_passes} min={1} max={5} />
			{/if}
			<SwitchField label="Shadows" bind:value={cfg.shadow_enabled} />
			{#if cfg.shadow_enabled}
				<SliderField label="Shadow size" bind:value={cfg.shadow_range} min={0} max={40} />
			{/if}
		</Group>

		{/if}
		{#if scope !== "windows"}
		<Group title={scope === "keyboard" ? "Keyboard layout" : scope === "mouse" ? "Windows" : "Mouse & touchpad"}>
			{#if scope === "all" || scope === "keyboard"}
			<SelectField label="Keyboard layout" bind:value={cfg.kb_layout} options={kbLayoutOptions} labels={KB_LABELS} />
			{/if}
			{#if scope === "all" || scope === "mouse"}
			<SelectField
				label="Focus"
				hint="Which window gets your typing."
				bind:value={followMouse}
				options={FOLLOW_MOUSE_MODES.map((m) => m.value)}
				labels={Object.fromEntries(FOLLOW_MOUSE_MODES.map((m) => [m.value, m.label]))}
			/>
			{#if scope === "all"}<SwitchField label="Natural scrolling" hint="Touchpad" bind:value={cfg.natural_scroll} />{/if}
			{/if}
		</Group>

		{/if}
		{#if scope === "all"}<Hint text="Applies as you change it." />{/if}
        {#if status}<p role="alert">{status}</p><button class="btn" onclick={()=>saveNow().catch(e=>status=String(e))}>Retry saving</button>{/if}
	{/if}
</ViewScaffold>

<style>
.window-preview{height:118px;display:grid;grid-template-columns:1.3fr 1fr;grid-template-rows:1fr 1fr;gap:var(--gi);padding:var(--go);background:var(--bg);border:1px solid var(--line);border-radius:12px;overflow:hidden}
.window-preview i{background:var(--surface-2);border:var(--bw) solid var(--accent);border-radius:var(--r);min-height:0}.window-preview i:first-child{grid-row:span 2}
</style>
