<script lang="ts">
	import { getCurrentWindow } from "@tauri-apps/api/window";

	function win() {
		return getCurrentWindow();
	}

	function close() {
		win().close();
	}
	function minimize() {
		win().minimize();
	}
	function toggleMax() {
		win().toggleMaximize();
	}
</script>

<div class="titlebar" data-tauri-drag-region>
	<div class="dots">
		<button class="dot close" aria-label="Close" onclick={close}></button>
		<button class="dot min" aria-label="Minimize" onclick={minimize}></button>
		<button class="dot max" aria-label="Maximize" onclick={toggleMax}></button>
	</div>
	<div class="name">Settings</div>
	<div class="grow"></div>
	<span class="kbd">Ctrl K</span>
</div>

<style>
	.titlebar {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 40px;
		padding: 0 14px;
		background: var(--bg-2, var(--bg));
		border-bottom: 1px solid var(--line, color-mix(in srgb, var(--fg) 8%, transparent));
		flex-shrink: 0;
	}

	.dots {
		display: flex;
		gap: 7px;
	}

	.dot {
		width: 11px;
		height: 11px;
		border-radius: 50%;
		border: none;
		padding: 0;
		background: #2a313c;
		cursor: pointer;
	}

	.dot.close:hover {
		background: var(--red);
	}
	.dot.min:hover,
	.dot.max:hover {
		background: var(--accent);
	}

	.name {
		font-size: 12px;
		color: var(--muted, color-mix(in oklab, var(--fg) 58%, transparent));
		letter-spacing: 0.02em;
		pointer-events: none;
	}

	.grow {
		flex: 1;
	}
</style>
