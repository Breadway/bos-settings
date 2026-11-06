<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import ViewScaffold from "$lib/components/ViewScaffold.svelte";
	import Group from "$lib/components/Group.svelte";
	import Hint from "$lib/components/Hint.svelte";
	import Row from "$lib/components/Row.svelte";
	import SwitchField from "$lib/components/SwitchField.svelte";
	import KeepSettings from "$lib/components/KeepSettings.svelte";
	import { uniqueBy } from "$lib/uniqueBy";
	import { pollWhileVisible } from "$lib/liveGate";
	import {
		type LiveMonitor,
		boxH,
		boxW,
		clone,
		groupMonitors,
		groupOf,
		mirrorChoices,
		withMirror,
		withMode,
		withPosition,
	} from "$lib/displayLayout";

	interface GuardStatus {
		pending: { id: number; seconds_left: number; seconds_total: number } | null;
		last: { id: number; kind: string; message: string } | null;
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
	// The change waiting for Keep or Revert. The deadline itself lives in the
	// backend; this only mirrors it so the countdown can be shown.
	let pending = $state<{ id: number; seconds: number } | null>(null);

	let live = $derived(monitors.find((m) => m.name === selected) ?? null);
	let groups = $derived(groupMonitors(monitors));
	let liveGroup = $derived(selected ? groupOf(monitors, selected) : null);
	let liveMirrors = $derived(live ? monitors.filter((m) => m.mirror_of === live.name) : []);
	let choices = $derived(live ? mirrorChoices(monitors, live.name) : []);
	// Never key the picker by a mode string the compositor may repeat.
	let modeChoices = $derived(uniqueBy(live?.available_modes ?? [], (mode) => mode));

	let view = $derived.by(() => {
		if (groups.length === 0) return { minX: 0, minY: 0, scale: 0.1 };
		let minX = Infinity;
		let minY = Infinity;
		let maxX = -Infinity;
		let maxY = -Infinity;
		for (const { source: m } of groups) {
			minX = Math.min(minX, m.x);
			minY = Math.min(minY, m.y);
			maxX = Math.max(maxX, m.x + boxW(m));
			maxY = Math.max(maxY, m.y + boxH(m));
		}
		const bw = Math.max(maxX - minX, 1);
		const bh = Math.max(maxY - minY, 1);
		const scale = Math.min((stageW - PAD * 2) / bw, (STAGE_H - PAD * 2) / bh);
		return { minX, minY, scale };
	});

	type Drag = { name: string; grabX: number; grabY: number; pointer: number; start: LiveMonitor[] };
	let drag = $state<Drag | null>(null);

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
		try {
			monitors = uniqueBy(await invoke<LiveMonitor[]>("get_live_monitors"), (m) => m.name);
		} catch (err) {
			message = `${err}`;
			return;
		}
		if (!selected || !monitors.some((m) => m.name === selected)) {
			selected = monitors[0]?.name ?? null;
		}
	}

	// A change can still be waiting if the page was left and opened again.
	async function adoptPending() {
		try {
			const status = await invoke<GuardStatus>("display_guard_status");
			if (status.pending) pending = { id: status.pending.id, seconds: Math.max(status.pending.seconds_left, 1) };
		} catch {
			// No guard state means nothing is waiting.
		}
	}

	onMount(() => {
		void refresh().then(adoptPending);
		const stopPoll = pollWhileVisible(() => {
			if (!drag && !applying && !pending) void refresh();
		}, 4000);
		const unlisten = listen<{ id: number; kind: string; message: string }>("display-guard-outcome", (e) => {
			if (pending && pending.id === e.payload.id) {
				pending = null;
				message = e.payload.message;
				void refresh();
			}
		});
		return () => {
			stopPoll();
			void unlisten.then((off) => off());
		};
	});

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

	function snap(name: string, x: number, y: number): { x: number; y: number } {
		const moving = monitors.find((m) => m.name === name);
		if (!moving) return { x, y };
		const mw = boxW(moving);
		const mh = boxH(moving);
		const thresh = Math.max(16, 18 / Math.max(view.scale, 0.01));
		let bestX = x;
		let bestY = y;
		let bestXd = thresh + 1;
		let bestYd = thresh + 1;
		for (const { source: o } of groups) {
			if (o.name === name) continue;
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

	function onDown(e: PointerEvent, m: LiveMonitor) {
		if (!stageEl || pending || applying) return;
		selected = m.name;
		const rect = stageEl.getBoundingClientRect();
		const px = e.clientX - rect.left;
		const py = e.clientY - rect.top;
		drag = { name: m.name, grabX: px - screenX(m), grabY: py - screenY(m), pointer: e.pointerId, start: clone(monitors) };
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
		const snapped = snap(drag.name, wx, wy);
		monitors = withPosition(monitors, drag.name, snapped.x, snapped.y);
	}

	async function onUp(e: PointerEvent) {
		if (!drag) return;
		if (e.pointerId !== drag.pointer && e.type !== "pointerleave") return;
		const { start, name } = drag;
		drag = null;
		const moved = monitors.find((m) => m.name === name);
		const was = start.find((m) => m.name === name);
		if (!moved || !was || (moved.x === was.x && moved.y === was.y)) return;
		await change(monitors, start);
	}

	// Every change goes through the backend guard: it snapshots the whole live
	// layout (mirrors included), applies the new one temporarily, checks what
	// the compositor really did, and reverts by itself unless it is kept. Only
	// Keep writes to disk. On failure the backend has already restored.
	async function change(next: LiveMonitor[], before: LiveMonitor[] = monitors) {
		if (next.length === 0 || pending || applying) return;
		applying = true;
		message = "";
		monitors = next;
		try {
			const status = await invoke<GuardStatus>("apply_display_layout", { monitors: next });
			if (status.pending) pending = { id: status.pending.id, seconds: Math.max(status.pending.seconds_left, 1) };
		} catch (err) {
			monitors = before;
			message = `${err}`;
		} finally {
			applying = false;
			await refresh();
		}
	}

	async function keepChanges() {
		const p = pending;
		pending = null;
		if (!p) return;
		try {
			await invoke("keep_display_layout", { id: p.id });
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
			const out = await invoke<{ message: string }>("revert_display_layout", { id: p.id });
			message = out.message;
		} catch (err) {
			message = `${err}`;
		}
		await refresh();
	}

	function mutate(fn: (m: LiveMonitor) => void) {
		if (!live) return;
		const next = clone(monitors);
		const m = next.find((o) => o.name === live!.name);
		if (!m) return;
		fn(m);
		return change(next);
	}

	async function setScale(value: number) {
		await mutate((m) => (m.scale = value));
	}

	async function setTransform(value: number) {
		await mutate((m) => (m.transform = value));
	}

	async function setVrr(on: boolean) {
		await mutate((m) => (m.vrr = on ? 1 : 0));
	}

	async function setMirror(source: string) {
		if (!live) return;
		await change(withMirror(monitors, live.name, source));
	}

	async function stopMirror(name: string) {
		await change(withMirror(monitors, name, ""));
	}

	// "1920x1200@60.00Hz" -> "1920 × 1200 · 60 Hz"; Hyprland lists the native mode first.
	function modeLabel(mode: string): string {
		const [res, hz] = mode.replace(/Hz$/i, "").split("@");
		const [w, h] = (res ?? "").split("x");
		const rate = Number(hz);
		return w && h ? `${w} × ${h}${rate ? ` · ${Number.isInteger(rate) ? rate : Math.round(rate * 100) / 100} Hz` : ""}` : mode;
	}

	async function setMode(modeStr: string) {
		if (!live) return;
		await change(withMode(monitors, live.name, modeStr));
	}
</script>

<ViewScaffold title="Displays" lede="Drag to arrange. Every change can be kept or reverted.">
	<Group title="Arrangement" wide>
		{#if groups.length === 0}
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
				{#each groups as g (g.source.name)}
					{@const m = g.source}
					<button
						type="button"
						class="mon"
						class:sel={liveGroup?.source.name === m.name}
						class:dragging={drag?.name === m.name}
						class:mirrored={g.mirrors.length > 0}
						style="left:{screenX(m)}px; top:{screenY(m)}px; width:{screenW(m)}px; height:{screenH(m)}px;"
						onpointerdown={(e) => onDown(e, m)}
					>
						<div class="screen"></div>
						<div class="chin">{[m.name, ...g.mirrors.map((x) => x.name)].join(" + ")}</div>
						{#if g.mirrors.length > 0}<div class="badge">Mirrored</div>{/if}
					</button>
				{/each}
			</div>
			{#if monitors.length > groups.length}
				<Hint text="Mirrored displays show the same picture, so they are drawn as one screen." />
			{/if}
			<Hint text={applying ? "Applying…" : "Drag a panel. You can keep or revert the change afterwards."} />
		{/if}
	</Group>

	{#if live}
		<Group title={live.name} preserveTitleCase>
			{#if monitors.length > 1}
				<Row label="Display">
					<div class="ui-seg">
						{#each monitors as m (m.name)}
							<button type="button" class:on={selected === m.name} onclick={() => (selected = m.name)}>
								{m.name}
							</button>
						{/each}
					</div>
				</Row>
			{/if}
			<Row label="Resolution">
				{#if modeChoices.length > 0}
					<select
						value={`${live.width}x${live.height}@${live.refresh.toFixed(2)}Hz`}
						disabled={applying || !!pending}
						onchange={(e) => setMode(e.currentTarget.value)}
					>
						{#each modeChoices as mode, i (mode)}
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
						<button
							type="button"
							class:on={nearestScale(live.scale) === s.value}
							disabled={applying || !!pending}
							onclick={() => setScale(s.value)}
						>
							{s.label}
						</button>
					{/each}
				</div>
			</Row>
			<Row label="Rotation">
				<div class="ui-seg">
					{#each TURNS as t (t.value)}
						<button
							type="button"
							class:on={live.transform % 4 === t.value}
							disabled={applying || !!pending}
							onclick={() => setTransform(t.value)}
						>
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
			{#if monitors.length > 1}
				{#if liveMirrors.length > 0}
					<Row label="Mirrored by" hint="These displays show the same picture as {live.name}.">
						<div class="ui-seg">
							{#each liveMirrors as m (m.name)}
								<button type="button" disabled={applying || !!pending} onclick={() => stopMirror(m.name)}>
									Stop mirroring {m.name}
								</button>
							{/each}
						</div>
					</Row>
				{:else}
					<Row
						label={live.mirror_of ? "Mirroring" : "Mirror another display"}
						hint={live.mirror_of ? `Showing the same picture as ${live.mirror_of}.` : undefined}
					>
						<select
							aria-label="Mirror another display"
							value={live.mirror_of}
							disabled={applying || !!pending}
							onchange={(e) => setMirror(e.currentTarget.value)}
						>
							<option value="">Use as a separate display</option>
							{#each choices as name (name)}
								<option value={name}>Mirror {name}</option>
							{/each}
						</select>
					</Row>
				{/if}
			{/if}
			{#if message}
				<Hint text={message} />
			{/if}
		</Group>
	{:else if message}
		<Hint text={message} />
	{/if}
</ViewScaffold>

{#if pending}
	{#key pending.id}
		<KeepSettings seconds={pending.seconds} onKeep={keepChanges} onRevert={revertChanges} />
	{/key}
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

	.mon.mirrored {
		border-style: dashed;
	}

	.badge {
		position: absolute;
		top: 8px;
		left: 8px;
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 10.5px;
		background: var(--accent);
		color: var(--bg);
		pointer-events: none;
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
