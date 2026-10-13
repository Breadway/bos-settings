<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import Row from "$lib/components/Row.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import KeepSettings from "$lib/components/KeepSettings.svelte";

	interface LiveMonitor {
		name: string;
		mode: string;
		x: number;
		y: number;
		width: number;
		height: number;
		refresh: number;
		scale: number;
		transform: number;
		available_modes: string[];
		vrr: number;
		mirror_of: string;
	}

	const SCALES = [
		{ label: "100%", value: 1 },
		{ label: "125%", value: 1.25 },
		{ label: "150%", value: 1.5 },
		{ label: "200%", value: 2 },
	];

	const TURNS = [
		{ label: "Landscape", value: 0 },
		{ label: "Right", value: 1 },
		{ label: "Upside down", value: 2 },
		{ label: "Left", value: 3 },
	];

	const PAD = 28;
	const STAGE_H = 300;

	let monitors = $state<LiveMonitor[]>([]);
	let selected = $state<string | null>(null);
	let stageEl: HTMLDivElement | undefined = $state();
	let stageW = $state(640);
	let message = $state("");
	let applying = $state(false);

	let live = $derived(monitors.find((m) => m.name === selected) ?? null);

	let view = $derived.by(() => {
		if (monitors.length === 0) return { minX: 0, minY: 0, scale: 0.1 };
		let minX = Infinity;
		let minY = Infinity;
		let maxX = -Infinity;
		let maxY = -Infinity;
		for (const m of monitors) {
			const w = boxW(m);
			const h = boxH(m);
			minX = Math.min(minX, m.x);
			minY = Math.min(minY, m.y);
			maxX = Math.max(maxX, m.x + w);
			maxY = Math.max(maxY, m.y + h);
		}
		const bw = Math.max(maxX - minX, 1);
		const bh = Math.max(maxY - minY, 1);
		const scale = Math.min((stageW - PAD * 2) / bw, (STAGE_H - PAD * 2) / bh);
		return { minX, minY, scale };
	});

	type Drag = { i: number; grabX: number; grabY: number; pointer: number };
	let drag = $state<Drag | null>(null);

	function rotated(m: LiveMonitor): boolean {
		return m.transform === 1 || m.transform === 3 || m.transform === 5 || m.transform === 7;
	}
	function boxW(m: LiveMonitor): number {
		return rotated(m) ? m.height : m.width;
	}
	function boxH(m: LiveMonitor): number {
		return rotated(m) ? m.width : m.height;
	}

	function screenX(m: LiveMonitor): number {
		return PAD + (m.x - view.minX) * view.scale;
	}
	function screenY(m: LiveMonitor): number {
		return PAD + (m.y - view.minY) * view.scale;
	}
	function screenW(m: LiveMonitor): number {
		return Math.max(48, boxW(m) * view.scale);
	}
	function screenH(m: LiveMonitor): number {
		return Math.max(32, boxH(m) * view.scale);
	}

	function nearestScale(v: number): number {
		return SCALES.reduce((best, s) => (Math.abs(s.value - v) < Math.abs(best - v) ? s.value : best), 1);
	}

	async function refresh() {
		monitors = await invoke<LiveMonitor[]>("get_live_monitors");
		if (!selected || !monitors.some((m) => m.name === selected)) {
			selected = monitors[0]?.name ?? null;
		}
	}

	onMount(refresh);

	$effect(() => {
		const el = stageEl;
		if (!el) return;
		stageW = el.clientWidth;
		const ro = new ResizeObserver(() => {
			stageW = el.clientWidth;
		});
		ro.observe(el);
		return () => ro.disconnect();
	});

	function snap(idx: number, x: number, y: number): { x: number; y: number } {
		const moving = monitors[idx];
		if (!moving) return { x, y };
		const mw = boxW(moving);
		const mh = boxH(moving);
		const thresh = Math.max(16, 18 / Math.max(view.scale, 0.01));
		let bestX = x;
		let bestY = y;
		let bestXd = thresh + 1;
		let bestYd = thresh + 1;
		for (let i = 0; i < monitors.length; i++) {
			if (i === idx) continue;
			const o = monitors[i];
			const ow = boxW(o);
			const oh = boxH(o);
			const xs = [x - o.x, x - (o.x + ow), x + mw - o.x, x + mw - (o.x + ow)];
			for (const d of xs) {
				const ad = Math.abs(d);
				if (ad < bestXd) {
					bestXd = ad;
					bestX = x - d;
				}
			}
			const ys = [y - o.y, y - (o.y + oh), y + mh - o.y, y + mh - (o.y + oh)];
			for (const d of ys) {
				const ad = Math.abs(d);
				if (ad < bestYd) {
					bestYd = ad;
					bestY = y - d;
				}
			}
		}
		return { x: Math.round(bestX), y: Math.round(bestY) };
	}

	function onDown(e: PointerEvent, i: number) {
		const m = monitors[i];
		if (!m || !stageEl) return;
		selected = m.name;
		const rect = stageEl.getBoundingClientRect();
		const px = e.clientX - rect.left;
		const py = e.clientY - rect.top;
		drag = { i, grabX: px - screenX(m), grabY: py - screenY(m), pointer: e.pointerId };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		e.preventDefault();
	}

	function onMove(e: PointerEvent) {
		if (!drag || !stageEl) return;
		const rect = stageEl.getBoundingClientRect();
		const px = e.clientX - rect.left - drag.grabX;
		const py = e.clientY - rect.top - drag.grabY;
		const wx = view.minX + (px - PAD) / view.scale;
		const wy = view.minY + (py - PAD) / view.scale;
		const snapped = snap(drag.i, wx, wy);
		monitors[drag.i].x = snapped.x;
		monitors[drag.i].y = snapped.y;
		monitors = [...monitors];
	}

	async function onUp(e: PointerEvent) {
		if (!drag) return;
		if (e.pointerId !== drag.pointer && e.type !== "pointerleave") return;
		drag = null;
		await apply();
	}

	// Layout moves (dragging a panel) are safe to apply and save at once; a
	// failure puts the real state back instead of leaving the dragged position.
	async function apply() {
		if (monitors.length === 0 || pending) return;
		applying = true;
		message = "";
		try {
			await invoke("apply_monitor_layout", { monitors });
		} catch (err) {
			message = `${err}`;
		} finally {
			applying = false;
			await refresh();
		}
	}

	function cloneMonitors(list: LiveMonitor[]): LiveMonitor[] {
		return list.map((m) => ({ ...m, available_modes: [...m.available_modes] }));
	}

	// Changes that can leave a screen unreadable (scale, rotation, mode, adaptive
	// sync) are applied live only, then kept or reverted from a countdown. The
	// optimistic UI state is rolled back if Hyprland rejects the change.
	let pending = $state<{ before: LiveMonitor[]; after: LiveMonitor[] } | null>(null);

	async function applyRisky(mutate: (m: LiveMonitor) => void) {
		if (!live || pending) return;
		const before = cloneMonitors(monitors);
		mutate(live);
		monitors = [...monitors];
		message = "";
		applying = true;
		try {
			await invoke("apply_monitor_layout", { monitors, persist: false });
			pending = { before, after: cloneMonitors(monitors) };
		} catch (err) {
			monitors = before;
			message = `${err}`;
		} finally {
			applying = false;
		}
	}

	async function keepChanges() {
		const p = pending;
		pending = null;
		if (!p) return;
		try {
			await invoke("persist_monitor_layout", { monitors: p.after });
		} catch (err) {
			message = `${err}`;
		}
		await refresh();
	}

	async function revertChanges() {
		const p = pending;
		pending = null;
		if (!p) return;
		try {
			await invoke("apply_monitor_layout", { monitors: p.before, persist: false });
		} catch (err) {
			message = `${err}`;
		}
		await refresh();
	}

	async function setScale(value: number) {
		await applyRisky((m) => (m.scale = value));
	}

	async function setTransform(value: number) {
		await applyRisky((m) => (m.transform = value));
	}

	async function setVrr(on: boolean) {
		await applyRisky((m) => (m.vrr = on ? 1 : 0));
	}

	// "1920x1200@60.00Hz" -> "1920 × 1200 · 60 Hz"; Hyprland lists the native mode first.
	function modeLabel(mode: string): string {
		const [res, hz] = mode.replace(/Hz$/i, "").split("@");
		const [w, h] = (res ?? "").split("x");
		const rate = Number(hz);
		return w && h ? `${w} × ${h}${rate ? ` · ${Number.isInteger(rate) ? rate : Math.round(rate * 100) / 100} Hz` : ""}` : mode;
	}

	async function setMode(modeStr: string) {
		const cleaned = modeStr.replace(/Hz$/i, "");
		const [res, hz] = cleaned.split("@");
		const [w, h] = (res ?? "").split("x");
		const width = Number(w);
		const height = Number(h);
		const rate = Number(hz);
		if (!width || !height) return;
		await applyRisky((m) => {
			m.width = width;
			m.height = height;
			if (rate) m.refresh = rate;
			m.mode = `${width}x${height} @ ${Math.round(m.refresh)}Hz`;
		});
	}
</script>

<ViewScaffold title="Displays" lede="Drag to arrange. Edges snap. Changes apply now.">
	<Group title="Arrangement" wide>
		{#if monitors.length === 0}
			<Hint text="No monitors detected." />
		{:else}
			<div
				class="mon-stage"
				role="application"
				aria-label="Monitor arrangement canvas. Drag to reorder monitors."
				bind:this={stageEl}
				onpointermove={onMove}
				onpointerup={onUp}
				onpointercancel={onUp}
			>
				{#each monitors as m, i (m.name)}
					<button
						type="button"
						class="mon"
						class:sel={selected === m.name}
						class:dragging={drag?.i === i}
						style="left:{screenX(m)}px; top:{screenY(m)}px; width:{screenW(m)}px; height:{screenH(m)}px;"
						onpointerdown={(e) => onDown(e, i)}
					>
						<div class="screen"></div>
						<div class="chin">{m.name}</div>
					</button>
				{/each}
			</div>
			<Hint text={applying ? "Applying…" : "Drag a panel. It sticks when you let go."} />
		{/if}
	</Group>

	{#if live}
		<Group title={live.name}>
			<Row label="Resolution">
				{#if live.available_modes.length > 0}
					<select value={`${live.width}x${live.height}@${live.refresh.toFixed(2)}Hz`} onchange={(e) => setMode(e.currentTarget.value)}>
						{#each live.available_modes as mode, i (mode)}
							<option value={mode}>{modeLabel(mode)}{i === 0 ? " (recommended)" : ""}</option>
						{/each}
					</select>
				{:else}
					<span class="mode">{live.mode}</span>
				{/if}
			</Row>
			<Row label="Scale">
				<div class="ui-seg">
					{#each SCALES as s (s.value)}
						<button type="button" class:on={nearestScale(live.scale) === s.value} onclick={() => setScale(s.value)}>
							{s.label}
						</button>
					{/each}
				</div>
			</Row>
			<Row label="Rotation">
				<div class="ui-seg">
					{#each TURNS as t (t.value)}
						<button type="button" class:on={live.transform % 4 === t.value} onclick={() => setTransform(t.value)}>
							{t.label}
						</button>
					{/each}
				</div>
			</Row>
			<SwitchField
				label="Adaptive sync"
				hint="Smoother games and video on screens that support it."
				bind:value={() => (live?.vrr ?? 0) > 0, (v) => setVrr(v)}
			/>
			{#if monitors.length>1}<Row label="Mirror another display"><select aria-label="Mirror another display" value={live.mirror_of} disabled={applying||!!pending} onchange={e=>applyRisky(m=>{m.mirror_of=e.currentTarget.value;})}><option value="">Use as a separate display</option>{#each monitors.filter(m=>m.name!==live!.name && !m.mirror_of) as monitor}<option value={monitor.name}>{monitor.name}</option>{/each}</select></Row>{/if}
			{#if message}
				<Hint text={message} />
			{/if}
		</Group>
	{/if}
</ViewScaffold>

{#if pending}
	<KeepSettings onKeep={keepChanges} onRevert={revertChanges} />
{/if}

<style>
	.mon-stage {
		position: relative;
		height: 300px;
		background: var(--bg);
		border-radius: 12px;
		overflow: hidden;
		touch-action: none;
		user-select: none;
	}

	.mon {
		position: absolute;
		border-radius: 10px;
		background: var(--surface-2, var(--surface));
		border: 2px solid color-mix(in srgb, var(--fg) 10%, transparent);
		box-shadow: 0 12px 28px #0005;
		display: flex;
		flex-direction: column;
		cursor: grab;
		padding: 0;
		color: inherit;
		overflow: hidden;
	}

	.mon.sel {
		border-color: var(--accent);
		z-index: 2;
	}

	.mon.dragging {
		cursor: grabbing;
		z-index: 3;
	}

	.screen {
		flex: 1;
		margin: 6px 6px 0;
		border-radius: 5px;
		background: linear-gradient(160deg, var(--surface) 10%, var(--accent) 140%);
		pointer-events: none;
	}

	.chin {
		height: 22px;
		display: grid;
		place-items: center;
		font-size: 11px;
		color: var(--muted);
		pointer-events: none;
	}

	.mode {
		color: var(--muted);
		font-size: 13px;
	}

	select {
		color-scheme: dark;
		background: var(--bg);
		color: var(--on-surface);
		border: 1px solid transparent;
		border-radius: 10px;
		padding: 6px 10px;
		max-width: 28ch;
	}
</style>
