<script lang="ts">
	import { onMount } from "svelte";

	// "Keep these display settings?": a modal countdown that reverts on its
	// own. The caller already applied the change live; `onKeep` persists it,
	// `onRevert` puts the previous state back. If nothing is chosen before the
	// countdown ends it reverts, because a bad scale or rotation can leave the
	// screen unreadable and unreachable.
	let {
		seconds = 15,
		onKeep,
		onRevert,
	}: { seconds?: number; onKeep: () => void; onRevert: () => void } = $props();

	let left = $state(0);
	let done = false;

	function finish(keep: boolean) {
		if (done) return;
		done = true;
		if (keep) onKeep();
		else onRevert();
	}

	onMount(() => {
		left = seconds;
		const iv = setInterval(() => {
			left -= 1;
			if (left <= 0) {
				clearInterval(iv);
				finish(false);
			}
		}, 1000);
		const onKey = (e: KeyboardEvent) => {
			if (e.key === "Escape") finish(false);
		};
		window.addEventListener("keydown", onKey);
		return () => {
			clearInterval(iv);
			window.removeEventListener("keydown", onKey);
		};
	});
</script>

<div class="scrim" role="presentation">
	<div class="sheet" role="alertdialog" aria-labelledby="keep-title" aria-describedby="keep-body">
		<h3 id="keep-title">Keep these display settings?</h3>
		<p id="keep-body">If you do not choose, BOS goes back to the previous settings.</p>
		<div class="count" aria-live="polite">{left}</div>
		<div class="acts">
			<button type="button" class="btn" onclick={() => finish(false)}>Revert</button>
			<button type="button" class="btn primary" onclick={() => finish(true)}>Keep changes</button>
		</div>
	</div>
</div>

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 200;
		display: grid;
		place-items: center;
		padding: 20px;
		background: color-mix(in srgb, var(--bg) 62%, transparent);
		backdrop-filter: blur(3px);
	}

	.sheet {
		width: min(440px, 100%);
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: 16px;
		padding: 20px;
		display: flex;
		flex-direction: column;
		gap: 12px;
		box-shadow: 0 30px 80px #0009;
	}

	h3 {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
	}

	p {
		margin: 0;
		color: var(--muted);
		font-size: 13px;
	}

	.count {
		text-align: center;
		font-size: 44px;
		font-weight: 300;
		line-height: 1;
		color: var(--accent);
	}

	.acts {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
		flex-wrap: wrap;
	}
</style>
